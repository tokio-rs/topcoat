use std::{
    any::{Any, TypeId},
    collections::hash_map::RandomState,
    future::{Future, poll_fn},
    hash::{BuildHasher, Hash},
    sync::{
        Mutex, OnceLock, PoisonError,
        atomic::{AtomicUsize, Ordering},
    },
};

use elsa::sync::FrozenMap;
use siphasher::sip128::{Hasher128, SipHasher13};
use smallvec::SmallVec;
use tokio::sync::OnceCell;

use super::recursion;
use crate::context::{CacheId, ContextRead, ContextTracker, Cx, replay_context_reads};

/// The per-request store backing `#[memoize]`.
///
/// An entry's identity is a 128 bit SipHash over the memoized function's `TypeId` and its
/// arguments, computed through the standard `Hash` trait. The hash is the whole key: the
/// cache keeps no owned copy of the arguments and runs no equality check. At 128 bits a
/// collision within a request is vanishingly unlikely, and the per-process random hash keys
/// keep colliding arguments from being crafted offline.
///
/// This trades on the `Hash` contract in one place: an impl that feeds identical bytes for
/// values its `Eq` distinguishes would make those values share an entry. Derived and standard
/// library impls distinguish everything they compare.
///
/// Each entry holds one cached value per context variant: the set of request-context
/// bindings read while computing it. A caller reuses a variant while its own reads resolve
/// to the same bindings and computes a new one when they differ.
#[derive(Default)]
#[doc(hidden)]
pub struct MemoizeCache {
    /// Cells are boxed so their addresses stay stable while the map grows, letting the cache
    /// hand out `&V` references whose lifetime is tied to the cache itself.
    entries: FrozenMap<u128, Box<dyn Any + Send + Sync>>,
}

impl MemoizeCache {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        MemoizeCache::default()
    }

    /// Hashes `(Marker, key)` into the 128 bit entry identity. `Marker` is the memoized
    /// function's type and partitions the cache so unrelated memoized functions cannot observe
    /// each other's entries even when they share a key shape.
    fn hash<Marker, K>(key: &K) -> u128
    where
        Marker: 'static,
        K: Hash,
    {
        let (key0, key1) = sip_keys();
        let mut hasher = SipHasher13::new_with_keys(key0, key1);
        TypeId::of::<Marker>().hash(&mut hasher);
        key.hash(&mut hasher);
        hasher.finish128().as_u128()
    }

    /// Returns a stable reference to the cell associated with `(Marker, key)`, creating a
    /// default cell on first access. The cell holds the context variants cached for the key.
    fn get_or_insert_cell<Marker, K, Cell>(&self, key: &K) -> &Cell
    where
        Marker: 'static,
        K: Hash,
        Cell: Default + Send + Sync + 'static,
    {
        let hash = Self::hash::<Marker, K>(key);
        let cell = match self.entries.get(&hash) {
            Some(cell) => cell,
            None => self.entries.insert_with(hash, || Box::new(Cell::default())),
        };
        cell.downcast_ref()
            .expect("entries of distinct types collided on a 128 bit memoize hash")
    }

    /// Runs `f(cx, params)` once per `(F, key, context variant)` and returns a reference to
    /// the cached result. `key` is the borrowed lookup key (e.g. `(&str,)`); `params` is what
    /// gets passed to `f` on a miss. Misses for one function and argument key are serialized.
    ///
    /// # Panics
    ///
    /// Panics if `f` panics, recursively initializes the same key, or an internal mutex is
    /// poisoned.
    pub fn memoize<'a, K, P, V, F>(&'a self, cx: &'a Cx, key: K, params: P, f: F) -> &'a V
    where
        K: Hash,
        V: Send + Sync + 'static,
        F: (FnOnce(&'a Cx, P) -> V) + 'static,
    {
        let slot = self.get_or_insert_cell::<F, _, SyncMemoSlot<V>>(&key);
        if let Some(value) = slot.find(cx) {
            return value;
        }

        slot.recursion.assert_not_recursive::<F>();
        let mut input = MemoizeInput::new(params, f);
        if let Some(value) = slot.initialize_first(cx, &mut input) {
            return value;
        }
        slot.initialize_overflow(cx, input)
    }

    /// Returns the already-computed value for `(F, key)`, or `None` if nothing has been
    /// memoized under that marker and key yet. Unlike [`memoize`](Self::memoize) this never
    /// inserts a cell or runs anything: `marker` is taken only to fix the partition type `F`
    /// (matching the function the value was memoized with) and is never called. When the key
    /// has context variants, this returns the first one.
    ///
    /// Only observes entries written by the synchronous [`memoize`](Self::memoize); the async
    /// variant uses a separate slot type and is not visible here.
    ///
    /// # Panics
    ///
    /// Panics if a stored cell cannot be downcast back to its expected type, indicating a
    /// value type mismatch between the caller and the function that originally memoized under
    /// the marker.
    #[allow(clippy::needless_pass_by_value)]
    #[track_caller]
    pub fn get<K, V, F>(&self, marker: F, key: K) -> Option<&V>
    where
        K: Hash,
        V: Send + Sync + 'static,
        F: 'static,
    {
        let _ = marker;
        let slot: &SyncMemoSlot<V> = self
            .entries
            .get(&Self::hash::<F, K>(&key))?
            .downcast_ref()
            .expect("memoized value type does not match the marker's return type");
        slot.first().map(|cached| &cached.value)
    }

    /// Async counterpart to [`memoize`](Self::memoize). Misses for one function and argument
    /// key are serialized with a cancellation-safe lock, so concurrent callers with matching
    /// context dependencies share one computation.
    ///
    /// # Panics
    ///
    /// Panics if `f` panics, recursively initializes the same key, or an internal memoization
    /// invariant is violated.
    pub async fn memoize_async<'a, K, P, V, F, Fut>(
        &'a self,
        cx: &'a Cx,
        key: K,
        params: P,
        f: F,
    ) -> &'a V
    where
        K: Hash,
        V: Send + Sync + 'static,
        F: (FnOnce(&'a Cx, P) -> Fut) + 'static,
        Fut: Future<Output = V>,
    {
        let slot = self.get_or_insert_cell::<F, _, AsyncMemoSlot<V>>(&key);
        if let Some(value) = slot.find(cx) {
            return value;
        }

        slot.recursion.assert_not_recursive::<F>();
        let mut input = MemoizeInput::new(params, f);
        if let Some(value) = slot.initialize_first(cx, &mut input).await {
            return value;
        }
        slot.initialize_overflow(cx, input).await
    }
}

impl std::fmt::Debug for MemoizeCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoizeCache").finish()
    }
}

/// Returns the process-wide random keys for the memoize hash function.
///
/// The keys must be shared by every hash the cache computes, but their value only has to be
/// unpredictable, so they are drawn once from the standard library's hasher randomness.
fn sip_keys() -> (u64, u64) {
    static KEYS: OnceLock<(u64, u64)> = OnceLock::new();
    *KEYS.get_or_init(|| {
        let entropy = RandomState::new();
        (entropy.hash_one(0u64), entropy.hash_one(1u64))
    })
}

// A once-cell initializer may lose a race and never run, so keep the call arguments available
// until a primary or overflow initializer actually consumes them.
struct MemoizeInput<P, F>(Option<(P, F)>);

impl<P, F> MemoizeInput<P, F> {
    #[inline]
    fn new(params: P, f: F) -> Self {
        Self(Some((params, f)))
    }

    #[inline]
    fn take(&mut self) -> (P, F) {
        self.0.take().expect("memoize input was already consumed")
    }
}

struct CachedValue<V> {
    last_context: AtomicUsize,
    value: V,
    reads: SmallVec<[ContextRead; 4]>,
}

impl<V> CachedValue<V> {
    fn new(cache_id: CacheId, value: V, reads: SmallVec<[ContextRead; 4]>) -> Self {
        Self {
            last_context: AtomicUsize::new(cache_id.get()),
            value,
            reads,
        }
    }

    #[inline]
    fn matches(&self, cx: &Cx) -> bool {
        let cache_id = cx.cache_id().get();
        if self.last_context.load(Ordering::Relaxed) == cache_id {
            return true;
        }
        if !cx.context_reads_match(&self.reads) {
            return false;
        }
        self.last_context.store(cache_id, Ordering::Relaxed);
        true
    }

    #[inline]
    fn value_for(&self, cx: &Cx) -> Option<&V> {
        if !self.matches(cx) {
            return None;
        }
        replay_context_reads(cx, &self.reads);
        Some(&self.value)
    }
}

struct MemoSlot<V, First, Gate> {
    first: First,
    overflow: OnceLock<Box<OverflowVariants<V, Gate>>>,
    recursion: recursion::Guard,
}

struct OverflowVariants<V, Gate> {
    values: boxcar::Vec<CachedValue<V>>,
    gate: Gate,
}

trait PrimaryVariant<V> {
    fn get(&self) -> Option<&CachedValue<V>>;
}

impl<V> PrimaryVariant<V> for OnceLock<CachedValue<V>> {
    fn get(&self) -> Option<&CachedValue<V>> {
        OnceLock::get(self)
    }
}

impl<V> PrimaryVariant<V> for OnceCell<CachedValue<V>> {
    fn get(&self) -> Option<&CachedValue<V>> {
        OnceCell::get(self)
    }
}

impl<V, First, Gate> MemoSlot<V, First, Gate>
where
    First: PrimaryVariant<V>,
{
    #[inline]
    fn find<'a>(&'a self, cx: &Cx) -> Option<&'a V> {
        self.first
            .get()
            .and_then(|cached| cached.value_for(cx))
            .or_else(|| {
                self.overflow
                    .get()?
                    .values
                    .iter()
                    .find_map(|(_, cached)| cached.value_for(cx))
            })
    }

    fn first(&self) -> Option<&CachedValue<V>> {
        self.first.get().or_else(|| {
            self.overflow
                .get()?
                .values
                .iter()
                .next()
                .map(|(_, cached)| cached)
        })
    }

    fn overflow(&self) -> &OverflowVariants<V, Gate>
    where
        Gate: Default,
    {
        self.overflow
            .get_or_init(|| Box::new(OverflowVariants::default()))
    }
}

impl<V, Gate> OverflowVariants<V, Gate> {
    fn insert(&self, cached: CachedValue<V>) -> &V {
        let index = self.values.push(cached);
        &self.values[index].value
    }
}

impl<V, Gate> Default for OverflowVariants<V, Gate>
where
    Gate: Default,
{
    fn default() -> Self {
        Self {
            values: boxcar::Vec::new(),
            gate: Gate::default(),
        }
    }
}

impl<V, First, Gate> Default for MemoSlot<V, First, Gate>
where
    First: Default,
    Gate: Default,
{
    fn default() -> Self {
        Self {
            first: First::default(),
            overflow: OnceLock::new(),
            recursion: recursion::Guard::default(),
        }
    }
}

type SyncMemoSlot<V> = MemoSlot<V, OnceLock<CachedValue<V>>, Mutex<()>>;
type AsyncMemoSlot<V> = MemoSlot<V, OnceCell<CachedValue<V>>, tokio::sync::Mutex<()>>;

impl<V> SyncMemoSlot<V> {
    #[inline]
    fn initialize_first<'a, P, F>(
        &'a self,
        cx: &'a Cx,
        input: &mut MemoizeInput<P, F>,
    ) -> Option<&'a V>
    where
        F: FnOnce(&'a Cx, P) -> V,
    {
        if self.first.get().is_some() {
            return None;
        }

        self.first
            .get_or_init(|| self.evaluate(cx, input))
            .value_for(cx)
    }

    #[inline]
    fn initialize_overflow<'a, P, F>(&'a self, cx: &'a Cx, mut input: MemoizeInput<P, F>) -> &'a V
    where
        F: FnOnce(&'a Cx, P) -> V,
    {
        let overflow = self.overflow();
        let _initialization = overflow.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(value) = self.find(cx) {
            return value;
        }

        overflow.insert(self.evaluate(cx, &mut input))
    }

    #[inline]
    fn evaluate<'a, P, F>(&self, cx: &'a Cx, input: &mut MemoizeInput<P, F>) -> CachedValue<V>
    where
        F: FnOnce(&'a Cx, P) -> V,
    {
        let (params, f) = input.take();
        let tracker = ContextTracker::new(cx);
        let value = tracker.scope(|| self.recursion.scope(|| f(cx, params)));
        CachedValue::new(cx.cache_id(), value, tracker.finish())
    }
}

impl<V> AsyncMemoSlot<V> {
    #[inline]
    async fn initialize_first<'a, P, F, Fut>(
        &'a self,
        cx: &'a Cx,
        input: &mut MemoizeInput<P, F>,
    ) -> Option<&'a V>
    where
        F: FnOnce(&'a Cx, P) -> Fut,
        Fut: Future<Output = V>,
    {
        if self.first.get().is_some() {
            return None;
        }

        self.first
            .get_or_init(|| self.evaluate(cx, input))
            .await
            .value_for(cx)
    }

    #[inline]
    async fn initialize_overflow<'a, P, F, Fut>(
        &'a self,
        cx: &'a Cx,
        mut input: MemoizeInput<P, F>,
    ) -> &'a V
    where
        F: FnOnce(&'a Cx, P) -> Fut,
        Fut: Future<Output = V>,
    {
        let overflow = self.overflow();
        let _initialization = overflow.gate.lock().await;
        if let Some(value) = self.find(cx) {
            return value;
        }

        overflow.insert(self.evaluate(cx, &mut input).await)
    }

    #[inline]
    async fn evaluate<'a, P, F, Fut>(
        &self,
        cx: &'a Cx,
        input: &mut MemoizeInput<P, F>,
    ) -> CachedValue<V>
    where
        F: FnOnce(&'a Cx, P) -> Fut,
        Fut: Future<Output = V>,
    {
        let (params, f) = input.take();
        let tracker = ContextTracker::new(cx);
        let mut future = std::pin::pin!(f(cx, params));
        // Re-enter the tracker and clear recursion ownership after every poll so sibling
        // futures can wait on this cell and the initializer can move between threads.
        let (value, reads) = poll_fn(move |task| {
            tracker
                .scope(|| self.recursion.scope(|| future.as_mut().poll(task)))
                .map(|value| (value, tracker.finish()))
        })
        .await;
        CachedValue::new(cx.cache_id(), value, reads)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;
    use crate::context::{CxTestBuilder, request_context};

    struct ScopedValue(i32);

    /// Returns a fresh counter with `'static` lifetime so closures that capture it can be
    /// `Copy + 'static` (the bounds `MemoizeCache::memoize` imposes on its function).
    fn counter() -> &'static AtomicUsize {
        Box::leak(Box::new(AtomicUsize::new(0)))
    }

    #[test]
    fn sync_same_key_runs_body_once() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = move |_: &Cx, (x, y): (i32, i32)| {
            n.fetch_add(1, Ordering::SeqCst);
            x + y
        };

        let a = cache.memoize(&cx, (&1i32, &2i32), (1, 2), f);
        let b = cache.memoize(&cx, (&1i32, &2i32), (1, 2), f);

        assert_eq!(*a, 3);
        assert_eq!(*b, 3);
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn sync_different_keys_run_body_per_key() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = move |_: &Cx, (x, y): (i32, i32)| {
            n.fetch_add(1, Ordering::SeqCst);
            x + y
        };

        cache.memoize(&cx, (&1i32, &2i32), (1, 2), f);
        cache.memoize(&cx, (&1i32, &3i32), (1, 3), f);
        cache.memoize(&cx, (&1i32, &2i32), (1, 2), f);

        assert_eq!(n.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn sync_different_functions_dont_collide() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n1 = counter();
        let n2 = counter();
        let f1 = move |_: &Cx, (x,): (i32,)| {
            n1.fetch_add(1, Ordering::SeqCst);
            x
        };
        let f2 = move |_: &Cx, (x,): (i32,)| {
            n2.fetch_add(1, Ordering::SeqCst);
            x * 10
        };

        let a = cache.memoize(&cx, (&1i32,), (1,), f1);
        let b = cache.memoize(&cx, (&1i32,), (1,), f2);

        assert_eq!(*a, 1);
        assert_eq!(*b, 10);
        assert_eq!(n1.load(Ordering::SeqCst), 1);
        assert_eq!(n2.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn sync_borrowed_str_key_dedupes_by_value() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = move |_: &Cx, (s,): (&str,)| {
            n.fetch_add(1, Ordering::SeqCst);
            s.to_owned()
        };

        // Two different `&str` slices with the same contents should share a cache entry.
        let s1 = String::from("alice");
        let s2 = String::from("alice");
        let a = cache.memoize(&cx, (s1.as_str(),), (s1.as_str(),), f);
        let b = cache.memoize(&cx, (s2.as_str(),), (s2.as_str(),), f);

        assert_eq!(a.as_str(), "alice");
        assert_eq!(b.as_str(), "alice");
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn sync_key_needs_only_hash() {
        /// A key type that is neither `Clone` nor `Eq`; hashing is the cache's only requirement.
        #[derive(Hash)]
        struct Token(u32);

        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = move |_: &Cx, (t,): (&Token,)| {
            n.fetch_add(1, Ordering::SeqCst);
            t.0
        };

        let a = *cache.memoize(&cx, (&Token(7),), (&Token(7),), f);
        let b = *cache.memoize(&cx, (&Token(7),), (&Token(7),), f);

        assert_eq!(a, 7);
        assert_eq!(b, 7);
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn sync_zero_arity_key() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = move |_: &Cx, (): ()| {
            n.fetch_add(1, Ordering::SeqCst);
            42
        };

        let a = cache.memoize(&cx, (), (), f);
        let b = cache.memoize(&cx, (), (), f);

        assert_eq!(*a, 42);
        assert_eq!(*b, 42);
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn sync_panicked_initializer_can_retry() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = move |_: &Cx, (): ()| {
            assert_ne!(n.fetch_add(1, Ordering::SeqCst), 0, "first attempt");
            42
        };

        let first = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cache.memoize(&cx, (), (), f);
        }));

        assert!(first.is_err());
        assert_eq!(*cache.memoize(&cx, (), (), f), 42);
    }

    #[test]
    fn get_observes_memoized_value() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let f = move |_: &Cx, (x,): (i32,)| x * 2;

        assert_eq!(cache.get::<_, i32, _>(f, (&3i32,)), None);
        cache.memoize(&cx, (&3i32,), (3,), f);
        assert_eq!(cache.get::<_, i32, _>(f, (&3i32,)), Some(&6));
        assert_eq!(cache.get::<_, i32, _>(f, (&4i32,)), None);
    }

    #[tokio::test]
    async fn async_concurrent_same_key_runs_body_once() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = async move |_: &Cx, (x, y): (i32, i32)| {
            n.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            x + y
        };

        let (a, b) = tokio::join!(
            cache.memoize_async(&cx, (&1i32, &2i32), (1, 2), f),
            cache.memoize_async(&cx, (&1i32, &2i32), (1, 2), f),
        );

        assert_eq!(*a, 3);
        assert_eq!(*b, 3);
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn async_different_keys_run_body_per_key() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = async move |_: &Cx, (x, y): (i32, i32)| {
            n.fetch_add(1, Ordering::SeqCst);
            x + y
        };

        cache.memoize_async(&cx, (&1i32, &2i32), (1, 2), f).await;
        cache.memoize_async(&cx, (&1i32, &3i32), (1, 3), f).await;
        cache.memoize_async(&cx, (&1i32, &2i32), (1, 2), f).await;

        assert_eq!(n.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn async_context_variants_are_cached_separately() {
        let cache = MemoizeCache::new();
        let cx = CxTestBuilder::new().request_context(ScopedValue(1)).build();
        let child = cx.with(ScopedValue(2));
        let n = counter();
        let f = async move |cx: &Cx, (): ()| {
            n.fetch_add(1, Ordering::SeqCst);
            request_context::<ScopedValue>(cx).0
        };

        assert_eq!(*cache.memoize_async(&cx, (), (), f).await, 1);
        assert_eq!(*cache.memoize_async(&child, (), (), f).await, 2);
        assert_eq!(*cache.memoize_async(&child, (), (), f).await, 2);
        assert_eq!(n.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn async_cancelled_initializer_can_retry() {
        let cache = MemoizeCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = async move |_: &Cx, (): ()| {
            if n.fetch_add(1, Ordering::SeqCst) == 0 {
                std::future::pending::<()>().await;
            }
            42
        };

        {
            let mut first = std::pin::pin!(cache.memoize_async(&cx, (), (), f));
            poll_fn(|task| {
                assert!(first.as_mut().poll(task).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
        }

        assert_eq!(*cache.memoize_async(&cx, (), (), f).await, 42);
    }
}
