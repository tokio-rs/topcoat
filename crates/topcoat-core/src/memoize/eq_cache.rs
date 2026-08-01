use std::{
    any::Any,
    collections::hash_map::RandomState,
    future::{Future, poll_fn},
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Condvar, Mutex},
};

use hashbrown::{Equivalent, HashMap};

use super::recursion;
use crate::context::{ContextRead, ContextTracker, Cx, replay_context_reads};

/// The per-request store backing `#[memoize]`.
///
/// Each function and owned argument key maps to a stable slot. A slot retains
/// every completed context variant for that key and serializes cache misses so
/// at most one caller runs the function body at a time.
#[derive(Default)]
#[doc(hidden)]
pub struct MemoizeEqCache {
    entries: Mutex<anymap3::Map<dyn Any + Send + Sync>>,
    slots: boxcar::Vec<Box<dyn Any + Send + Sync + 'static>>,
}

impl MemoizeEqCache {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn get_or_insert_slot<Marker, K, Slot>(&self, key: K) -> &Slot
    where
        Marker: 'static,
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        Slot: Default + Send + Sync + 'static,
    {
        let index = {
            let mut entries = self.entries.lock().unwrap();
            let cache = entries
                .entry::<MarkedHashMap<(Marker, Slot), <MemoizeKey<K> as ToOwnedKey>::Owned, usize>>()
                .or_insert_with(MarkedHashMap::new);

            if let Some(&index) = cache.get(&MemoizeKey(key)) {
                index
            } else {
                let index = self.slots.push(Box::new(Slot::default()));
                cache.insert(MemoizeKey(key).to_owned_key(), index);
                index
            }
        };
        self.slots[index].downcast_ref().unwrap()
    }

    /// Returns the matching result for `(F, key)`, or computes and stores a new
    /// context variant.
    ///
    /// Calls with a completed matching variant do not acquire the
    /// initialization gate. Cache misses for one function and argument key are
    /// serialized; other keys and functions remain independent.
    ///
    /// # Panics
    ///
    /// Panics for recursive initialization with the same key or if internal
    /// cache synchronization has been poisoned.
    pub fn memoize<'a, K, P, V, F>(&'a self, cx: &'a Cx, key: K, params: P, f: F) -> &'a V
    where
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: (FnOnce(&'a Cx, P) -> V) + 'static,
    {
        let slot = self.get_or_insert_slot::<F, _, SyncMemoSlot<V>>(key);
        if let Some(value) = slot.find(cx) {
            return value;
        }

        slot.recursion.assert_not_recursive::<F>();
        let mut state = slot.state.lock().unwrap();
        loop {
            if let Some(value) = slot.find(cx) {
                return value;
            }
            if !state.running {
                state.running = true;
                break;
            }
            slot.recursion.assert_not_recursive::<F>();
            state = slot.ready.wait(state).unwrap();
        }
        drop(state);

        let initialization = SyncInitialization::new(slot);
        let tracker = ContextTracker::new(cx);
        let value = tracker.scope(|| slot.recursion.scope(|| f(cx, params)));
        let index = slot.values.push(CachedValue {
            value,
            reads: tracker.finish(),
        });
        initialization.finish();
        &slot.values[index].value
    }

    /// Returns the matching completed synchronous value for `(F, key)`.
    ///
    /// This never inserts a slot or runs the memoized body. `marker` is used
    /// only to select the function partition.
    ///
    /// # Panics
    ///
    /// Panics if internal cache synchronization has been poisoned or stored
    /// type metadata does not match the requested marker, key, and value.
    #[allow(clippy::needless_pass_by_value)]
    pub fn get<K, V, F>(&self, cx: &Cx, marker: F, key: K) -> Option<&V>
    where
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: 'static,
    {
        let _ = marker;
        let index = {
            let entries = self.entries.lock().unwrap();
            let cache = entries.get::<MarkedHashMap<
                (F, SyncMemoSlot<V>),
                <MemoizeKey<K> as ToOwnedKey>::Owned,
                usize,
            >>()?;
            *cache.get(&MemoizeKey(key))?
        };
        let slot: &SyncMemoSlot<V> = self.slots[index].downcast_ref().unwrap();
        slot.find(cx)
    }

    /// Async counterpart to [`memoize`](Self::memoize).
    ///
    /// The body is tracked during each poll. The initialization lock is
    /// cancellation safe: dropping or unwinding the future stores no value and
    /// lets the next waiter retry.
    ///
    /// # Panics
    ///
    /// Panics for recursive initialization with the same key or if internal
    /// cache synchronization has been poisoned.
    pub async fn memoize_async<'a, K, P, V, F, Fut>(
        &'a self,
        cx: &'a Cx,
        key: K,
        params: P,
        f: F,
    ) -> &'a V
    where
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: (FnOnce(&'a Cx, P) -> Fut) + 'static,
        Fut: Future<Output = V>,
    {
        let slot = self.get_or_insert_slot::<F, _, AsyncMemoSlot<V>>(key);
        if let Some(value) = slot.find(cx) {
            return value;
        }

        slot.recursion.assert_not_recursive::<F>();
        let _initialization = slot.gate.lock().await;
        if let Some(value) = slot.find(cx) {
            return value;
        }

        let tracker = ContextTracker::new(cx);
        let mut future = std::pin::pin!(f(cx, params));
        let value =
            poll_fn(|task| tracker.scope(|| slot.recursion.scope(|| future.as_mut().poll(task))))
                .await;
        let index = slot.values.push(CachedValue {
            value,
            reads: tracker.finish(),
        });
        &slot.values[index].value
    }
}

impl std::fmt::Debug for MemoizeEqCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoizeEqCache").finish()
    }
}

struct CachedValue<V> {
    value: V,
    reads: Vec<ContextRead>,
}

struct MemoSlot<V, Gate> {
    values: boxcar::Vec<CachedValue<V>>,
    gate: Gate,
    recursion: recursion::Guard,
}

impl<V, Gate> MemoSlot<V, Gate> {
    fn find<'a>(&'a self, cx: &Cx) -> Option<&'a V> {
        self.values.iter().find_map(|(_, cached)| {
            if cx.context_reads_match(&cached.reads) {
                replay_context_reads(cx, &cached.reads);
                Some(&cached.value)
            } else {
                None
            }
        })
    }
}

impl<V, Gate> Default for MemoSlot<V, Gate>
where
    Gate: Default,
{
    fn default() -> Self {
        Self {
            values: boxcar::Vec::new(),
            gate: Gate::default(),
            recursion: recursion::Guard::default(),
        }
    }
}

#[derive(Default)]
struct SyncGateState {
    running: bool,
}

struct SyncMemoSlot<V> {
    values: boxcar::Vec<CachedValue<V>>,
    state: Mutex<SyncGateState>,
    ready: Condvar,
    recursion: recursion::Guard,
}

impl<V> SyncMemoSlot<V> {
    fn find<'a>(&'a self, cx: &Cx) -> Option<&'a V> {
        self.values.iter().find_map(|(_, cached)| {
            if cx.context_reads_match(&cached.reads) {
                replay_context_reads(cx, &cached.reads);
                Some(&cached.value)
            } else {
                None
            }
        })
    }
}

impl<V> Default for SyncMemoSlot<V> {
    fn default() -> Self {
        Self {
            values: boxcar::Vec::new(),
            state: Mutex::new(SyncGateState::default()),
            ready: Condvar::new(),
            recursion: recursion::Guard::default(),
        }
    }
}

struct SyncInitialization<'a, V> {
    slot: &'a SyncMemoSlot<V>,
    armed: bool,
}

impl<'a, V> SyncInitialization<'a, V> {
    fn new(slot: &'a SyncMemoSlot<V>) -> Self {
        Self { slot, armed: true }
    }

    fn finish(mut self) {
        self.release();
        self.armed = false;
    }

    fn release(&self) {
        self.slot.state.lock().unwrap().running = false;
        self.slot.ready.notify_all();
    }
}

impl<V> Drop for SyncInitialization<'_, V> {
    fn drop(&mut self) {
        if self.armed {
            self.release();
        }
    }
}

type AsyncMemoSlot<V> = MemoSlot<V, tokio::sync::Mutex<()>>;

/// A `HashMap` tagged by `Marker`, keeping functions with the same argument
/// shape in separate entries.
struct MarkedHashMap<Marker, K, V> {
    inner: HashMap<K, V, RandomState>,
    marker: PhantomData<fn() -> Marker>,
}

impl<Marker, K, V> MarkedHashMap<Marker, K, V> {
    fn new() -> Self {
        Self {
            inner: HashMap::with_hasher(RandomState::new()),
            marker: PhantomData,
        }
    }
}

impl<Marker, K, V> Deref for MarkedHashMap<Marker, K, V> {
    type Target = HashMap<K, V, RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<Marker, K, V> DerefMut for MarkedHashMap<Marker, K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A newtype around the borrowed argument tuple used for cache lookup.
#[doc(hidden)]
#[derive(Hash)]
pub struct MemoizeKey<T>(T);

/// Converts a borrowed memoization key to its owned stored form.
#[doc(hidden)]
pub trait ToOwnedKey {
    type Owned;
    fn to_owned_key(&self) -> Self::Owned;
}

macro_rules! impl_tuple {
    ($(($kty:ident, $qty:ident, $accessor:tt)),*) => {
        impl<$($kty, $qty),*> Equivalent<($($kty,)*)> for MemoizeKey<($(&$qty,)*)>
        where
            $($qty: ?Sized + Equivalent<$kty>,)*
        {
            fn equivalent(&self, key: &($($kty,)*)) -> bool {
                $(self.0.$accessor.equivalent(&key.$accessor))&&*
            }
        }

        impl<$($qty),*> ToOwnedKey for MemoizeKey<($(&$qty,)*)>
        where
            $($qty: ?Sized + ToOwned,)*
        {
            type Owned = ($($qty::Owned,)*);

            fn to_owned_key(&self) -> Self::Owned {
                ($(self.0.$accessor.to_owned(),)*)
            }
        }
    };
}

#[rustfmt::skip]
mod impls {
    use super::{Equivalent, MemoizeKey, ToOwnedKey};

    impl Equivalent<()> for MemoizeKey<()> {
        fn equivalent(&self, _key: &()) -> bool { true }
    }
    impl ToOwnedKey for MemoizeKey<()> {
        type Owned = ();
        fn to_owned_key(&self) -> Self::Owned {}
    }

    impl_tuple!((K1, Q1, 0));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4), (K6, Q6, 5));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4), (K6, Q6, 5), (K7, Q7, 6));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4), (K6, Q6, 5), (K7, Q7, 6), (K8, Q8, 7));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4), (K6, Q6, 5), (K7, Q7, 6), (K8, Q8, 7), (K9, Q9, 8));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4), (K6, Q6, 5), (K7, Q7, 6), (K8, Q8, 7), (K9, Q9, 8), (K10, Q10, 9));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4), (K6, Q6, 5), (K7, Q7, 6), (K8, Q8, 7), (K9, Q9, 8), (K10, Q10, 9), (K11, Q11, 10));
    impl_tuple!((K1, Q1, 0), (K2, Q2, 1), (K3, Q3, 2), (K4, Q4, 3), (K5, Q5, 4), (K6, Q6, 5), (K7, Q7, 6), (K8, Q8, 7), (K9, Q9, 8), (K10, Q10, 9), (K11, Q11, 10), (K12, Q12, 11));
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    fn counter() -> &'static AtomicUsize {
        Box::leak(Box::new(AtomicUsize::new(0)))
    }

    #[test]
    fn sync_same_key_runs_body_once() {
        let cache = MemoizeEqCache::new();
        let cx = Cx::default();
        let calls = counter();
        let f = move |_: &Cx, (x, y): (i32, i32)| {
            calls.fetch_add(1, Ordering::SeqCst);
            x + y
        };

        let first = cache.memoize(&cx, (&1, &2), (1, 2), f);
        let second = cache.memoize(&cx, (&1, &2), (1, 2), f);

        assert_eq!(*first, 3);
        assert_eq!(*second, 3);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn async_same_key_runs_body_once() {
        let cache = MemoizeEqCache::new();
        let cx = Cx::default();
        let calls = counter();
        let f = move |_: &Cx, (x, y): (i32, i32)| async move {
            calls.fetch_add(1, Ordering::SeqCst);
            x + y
        };

        let first = cache.memoize_async(&cx, (&1, &2), (1, 2), f).await;
        let second = cache.memoize_async(&cx, (&1, &2), (1, 2), f).await;

        assert_eq!(*first, 3);
        assert_eq!(*second, 3);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
