use std::{
    any::Any,
    collections::hash_map::RandomState,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Condvar, Mutex},
};

use hashbrown::{Equivalent, HashMap};
use tokio::sync::Notify;

use crate::context::{ContextRead, Cx};

/// The per-request store backing `#[memoize]`.
///
/// Each function and owned argument key maps to a stable slot. A slot retains
/// every completed context variant for that key and serializes computation so
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

    /// Looks up a synchronous result or claims its key for execution.
    ///
    /// # Panics
    ///
    /// Panics if internal memoization state was poisoned by another panic.
    pub fn memoize<Marker, K, V>(&self, cx: &Cx, key: K) -> MemoizeEntry<'_, V, SyncVacant<'_, V>>
    where
        Marker: 'static,
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
    {
        let revision = cx.memo_revision();
        let slot = self.get_or_insert_slot::<Marker, _, SyncMemoSlot<V>>(key);
        let mut state = slot.state.lock().unwrap();

        loop {
            if let Some(value) = slot.find(&state, cx, revision) {
                return MemoizeEntry::Occupied(value);
            }
            if !state.running {
                state.running = true;
                drop(state);
                return MemoizeEntry::Vacant(SyncVacant {
                    inner: Vacant {
                        slot,
                        cx: cx.start_memo(revision),
                        armed: true,
                    },
                });
            }
            state = slot.ready.wait(state).unwrap();
        }
    }

    /// Returns a synchronous value for `key`, without running its body. When
    /// the key has context variants, this returns the first one.
    ///
    /// # Panics
    ///
    /// Panics if internal memoization state was poisoned by another panic.
    #[allow(clippy::needless_pass_by_value)]
    pub fn get<K, V, Marker>(&self, marker: Marker, key: K) -> Option<&V>
    where
        Marker: 'static,
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
    {
        let _ = marker;
        let index = {
            let entries = self.entries.lock().unwrap();
            let cache = entries.get::<MarkedHashMap<
                (Marker, SyncMemoSlot<V>),
                <MemoizeKey<K> as ToOwnedKey>::Owned,
                usize,
            >>()?;
            *cache.get(&MemoizeKey(key))?
        };
        let slot: &SyncMemoSlot<V> = self.slots[index].downcast_ref().unwrap();
        let state = slot.state.lock().unwrap();
        (state.completed > 0).then(|| &slot.values[0].value)
    }

    /// Looks up an asynchronous result or claims its key for execution.
    /// Waiters keep the context revision captured before they began waiting.
    ///
    /// # Panics
    ///
    /// Panics if internal memoization state was poisoned by another panic.
    pub async fn memoize_async<Marker, K, V>(
        &self,
        cx: &Cx,
        key: K,
    ) -> MemoizeEntry<'_, V, AsyncVacant<'_, V>>
    where
        Marker: 'static,
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
    {
        let revision = cx.memo_revision();
        let slot = self.get_or_insert_slot::<Marker, _, AsyncMemoSlot<V>>(key);

        loop {
            let notified = slot.ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();

            {
                let mut state = slot.state.lock().unwrap();
                if let Some(value) = slot.find(&state, cx, revision) {
                    return MemoizeEntry::Occupied(value);
                }
                if !state.running {
                    state.running = true;
                    return MemoizeEntry::Vacant(AsyncVacant {
                        inner: Vacant {
                            slot,
                            cx: cx.start_memo(revision),
                            armed: true,
                        },
                    });
                }
            }

            notified.await;
        }
    }
}

impl std::fmt::Debug for MemoizeEqCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoizeEqCache").finish()
    }
}

/// The outcome of checking a memoized function and argument key.
#[doc(hidden)]
pub enum MemoizeEntry<'a, V, Vacant> {
    Occupied(&'a V),
    Vacant(Vacant),
}

struct Vacant<'a, V, Ready: WakeAll> {
    slot: &'a MemoSlot<V, Ready>,
    cx: Cx,
    armed: bool,
}

impl<'a, V, Ready> Vacant<'a, V, Ready>
where
    V: Send + Sync + 'static,
    Ready: WakeAll,
{
    fn cx(&self) -> &Cx {
        &self.cx
    }

    fn insert(mut self, value: V) -> &'a V {
        let index = self.slot.values.push(MemoizedValue {
            value,
            reads: self.cx.finish_memo(),
        });
        {
            let mut state = self.slot.state.lock().unwrap();
            debug_assert_eq!(index, state.completed);
            state.completed += 1;
            state.running = false;
        }
        self.armed = false;
        self.slot.ready.notify_all();
        &self.slot.values[index].value
    }
}

impl<V, Ready> Drop for Vacant<'_, V, Ready>
where
    Ready: WakeAll,
{
    fn drop(&mut self) {
        if self.armed {
            self.slot.state.lock().unwrap().running = false;
            self.slot.ready.notify_all();
        }
    }
}

macro_rules! impl_vacant {
    ($name:ident, $ready:ty) => {
        /// A claimed memoization key.
        #[doc(hidden)]
        pub struct $name<'a, V> {
            inner: Vacant<'a, V, $ready>,
        }

        impl<'a, V> $name<'a, V>
        where
            V: Send + Sync + 'static,
        {
            #[must_use]
            pub fn cx(&self) -> &Cx {
                self.inner.cx()
            }

            /// Publishes the computed value and releases callers waiting on this key.
            ///
            /// # Panics
            ///
            /// Panics if internal memoization state was poisoned by another panic.
            #[must_use]
            pub fn insert(self, value: V) -> &'a V {
                self.inner.insert(value)
            }
        }
    };
}

impl_vacant!(SyncVacant, Condvar);
impl_vacant!(AsyncVacant, Notify);

struct MemoizedValue<V> {
    value: V,
    reads: Vec<ContextRead>,
}

#[derive(Default)]
struct MemoSlotState {
    completed: usize,
    running: bool,
}

struct MemoSlot<V, Ready> {
    values: boxcar::Vec<MemoizedValue<V>>,
    state: Mutex<MemoSlotState>,
    ready: Ready,
}

impl<V, Ready> MemoSlot<V, Ready> {
    fn find<'a>(&'a self, state: &MemoSlotState, cx: &Cx, revision: u64) -> Option<&'a V> {
        (0..state.completed).find_map(|index| {
            let memo = &self.values[index];
            if cx.context_reads_match(revision, &memo.reads) {
                cx.record_context_reads(&memo.reads);
                Some(&memo.value)
            } else {
                None
            }
        })
    }
}

impl<V, Ready> Default for MemoSlot<V, Ready>
where
    Ready: Default,
{
    fn default() -> Self {
        Self {
            values: boxcar::Vec::new(),
            state: Mutex::new(MemoSlotState::default()),
            ready: Ready::default(),
        }
    }
}

type SyncMemoSlot<V> = MemoSlot<V, Condvar>;
type AsyncMemoSlot<V> = MemoSlot<V, Notify>;

trait WakeAll {
    fn notify_all(&self);
}

impl WakeAll for Condvar {
    fn notify_all(&self) {
        Condvar::notify_all(self);
    }
}

impl WakeAll for Notify {
    fn notify_all(&self) {
        self.notify_waiters();
    }
}

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

    struct Add;

    #[test]
    fn sync_same_context_variant_reuses_value() {
        let cache = MemoizeEqCache::new();
        let cx = Cx::default();
        let calls = AtomicUsize::new(0);

        for _ in 0..2 {
            match cache.memoize::<Add, _, i32>(&cx, (&1, &2)) {
                MemoizeEntry::Occupied(value) => assert_eq!(*value, 3),
                MemoizeEntry::Vacant(vacant) => {
                    calls.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(*vacant.insert(3), 3);
                }
            }
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn async_same_context_variant_reuses_value() {
        let cache = MemoizeEqCache::new();
        let cx = Cx::default();
        let calls = AtomicUsize::new(0);

        for _ in 0..2 {
            match cache.memoize_async::<Add, _, i32>(&cx, (&1, &2)).await {
                MemoizeEntry::Occupied(value) => assert_eq!(*value, 3),
                MemoizeEntry::Vacant(vacant) => {
                    calls.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(*vacant.insert(3), 3);
                }
            }
        }

        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
