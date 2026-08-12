use std::{
    any::{Any, TypeId},
    collections::hash_map::RandomState,
    future::{Future, poll_fn},
    hash::{BuildHasher, Hash},
    sync::OnceLock,
};

use elsa::sync::FrozenMap;
use siphasher::sip128::{Hasher128, SipHasher13};
use tokio::sync::OnceCell;

use super::recursion;
use crate::context::Cx;

/// A cached value and the guard detecting recursive initialization of it.
#[derive(Default)]
struct MemoizeCell<T> {
    value: T,
    recursion: recursion::Guard,
}

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
    ///
    /// TODO: the identity does not yet cover the `BindingId`s of the request context bindings
    /// in effect, so a memoized function that reads a value shadowed via `Cx::with` shares its
    /// entry across scopes and can observe a result computed under different bindings.
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
    /// default cell on first access.
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

    /// Runs `f(cx, params)` at most once per `(F, key)` and returns a reference to the cached
    /// result. `key` is the borrowed lookup key (e.g. `(&str,)`); `params` is what gets passed
    /// to `f` on a miss.
    pub fn memoize<'a, K, P, V, F>(&'a self, cx: &'a Cx, key: K, params: P, f: F) -> &'a V
    where
        K: Hash,
        V: Send + Sync + 'static,
        F: (FnOnce(&'a Cx, P) -> V) + 'static,
    {
        let cell = self.get_or_insert_cell::<F, _, MemoizeCell<OnceLock<V>>>(&key);
        if let Some(value) = cell.value.get() {
            return value;
        }
        cell.recursion.assert_not_recursive::<F>();
        cell.value
            .get_or_init(|| cell.recursion.scope(|| f(cx, params)))
    }

    /// Returns the already-computed value for `(F, key)`, or `None` if nothing has been
    /// memoized under that marker and key yet. Unlike [`memoize`](Self::memoize) this never
    /// inserts a cell or runs anything: `marker` is taken only to fix the partition type `F`
    /// (matching the function the value was memoized with) and is never called.
    ///
    /// Only observes entries written by the synchronous [`memoize`](Self::memoize); the async
    /// variant stores its cells as `OnceCell<V>` and is not visible here.
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
        let cell: &MemoizeCell<OnceLock<V>> = self
            .entries
            .get(&Self::hash::<F, K>(&key))?
            .downcast_ref()
            .expect("memoized value type does not match the marker's return type");
        cell.value.get()
    }

    /// Async counterpart to [`memoize`](Self::memoize). Concurrent callers with the same key
    /// share a single in-flight future via `tokio::sync::OnceCell`.
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
        let cell = self.get_or_insert_cell::<F, _, MemoizeCell<OnceCell<V>>>(&key);
        if let Some(value) = cell.value.get() {
            return value;
        }
        cell.recursion.assert_not_recursive::<F>();
        cell.value
            .get_or_init(|| async {
                let mut future = std::pin::pin!(cell.recursion.scope(|| f(cx, params)));
                // Clear ownership after every poll so sibling futures can wait on this cell and
                // the initializer can move between executor threads.
                poll_fn(|task| cell.recursion.scope(|| future.as_mut().poll(task))).await
            })
            .await
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

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

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
