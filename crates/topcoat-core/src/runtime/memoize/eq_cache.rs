use std::{
    any::Any,
    collections::hash_map::RandomState,
    future::Future,
    hash::Hash,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::{Mutex, OnceLock},
};

use hashbrown::{Equivalent, HashMap};
use tokio::sync::OnceCell;

use crate::runtime::context::Cx;

/// The per-request store backing `#[memoize]`.
///
/// This cache variant holds an owned instance of the key, allowing it to guarantee equality
/// through the `Eq` trait.
///
/// `entries` maps an `(F, K)` shape to an index into `values` via [`anymap3::Map`], where `F` is
/// the memoized function (used as a marker type to keep different functions' caches disjoint) and
/// `K` is the owned key type. `values` holds the actual cells (`OnceLock<V>` / `OnceCell<V>`)
/// behind a stable address so we can hand out `&V` references whose lifetime is tied to the cache.
#[derive(Default)]
#[doc(hidden)]
pub struct MemoizeEqCache {
    entries: Mutex<anymap3::Map<dyn Any + Send + Sync>>,
    values: boxcar::Vec<Box<dyn Any + Send + Sync + 'static>>,
}

impl MemoizeEqCache {
    #[must_use]
    pub fn new() -> Self {
        MemoizeEqCache::default()
    }

    /// Returns a stable reference to the cell associated with `(Marker, key)`, creating a default
    /// cell on first access. `Marker` is the function type and partitions the cache so unrelated
    /// memoized functions cannot observe each other's entries even when they share a key shape.
    fn get_or_insert_cell<Marker, K, Cell>(&self, key: K) -> &Cell
    where
        Marker: 'static,
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        Cell: Default + Send + Sync + 'static,
    {
        let index = {
            let mut guard = self.entries.lock().unwrap();
            let cache = guard
                .entry::<MarkedHashMap<Marker, <MemoizeKey<K> as ToOwnedKey>::Owned, usize>>()
                .or_insert_with(|| MarkedHashMap::new());

            // Look up using the borrowed key via `Equivalent` to avoid cloning the arguments on
            // cache hits; only clone into an owned key when inserting.
            if let Some(&index) = cache.get(&MemoizeKey(key)) {
                index
            } else {
                let index = self.values.push(Box::new(Cell::default()));
                let key_owned = MemoizeKey(key).to_owned_key();
                cache.insert(key_owned, index);
                index
            }
        };
        self.values.get(index).unwrap().downcast_ref().unwrap()
    }

    /// Runs `f(cx, params)` at most once per `(F, key)` and returns a reference to the cached
    /// result. `key` is the borrowed lookup key (e.g. `(&str,)`) used to avoid cloning on cache
    /// hits; `params` is what gets passed to `f` on a miss.
    pub fn memoize<'a, K, P, V, F>(&'a self, cx: &'a Cx, key: K, params: P, f: F) -> &'a V
    where
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: (FnOnce(&'a Cx, P) -> V) + 'static,
    {
        let cell = self.get_or_insert_cell::<F, _, OnceLock<V>>(key);
        cell.get_or_init(|| f(cx, params))
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
    /// Panics if the internal mutex is poisoned, or if a stored cell cannot be
    /// downcast back to `OnceLock<V>` (which indicates a marker/type mismatch
    /// between the caller and the function that originally memoized the value).
    #[allow(clippy::needless_pass_by_value)]
    pub fn get<K, V, F>(&self, marker: F, key: K) -> Option<&V>
    where
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: 'static,
    {
        let _ = marker;
        let index = {
            let guard = self.entries.lock().unwrap();
            let cache =
                guard.get::<MarkedHashMap<F, <MemoizeKey<K> as ToOwnedKey>::Owned, usize>>()?;
            *cache.get(&MemoizeKey(key))?
        };
        let cell: &OnceLock<V> = self.values.get(index).unwrap().downcast_ref().unwrap();
        cell.get()
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
        K: Copy,
        MemoizeKey<K>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<K> as ToOwnedKey>::Owned>,
        <MemoizeKey<K> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: (FnOnce(&'a Cx, P) -> Fut) + 'static,
        Fut: Future<Output = V>,
    {
        let cell = self.get_or_insert_cell::<F, _, OnceCell<V>>(key);
        cell.get_or_init(|| async { f(cx, params).await }).await
    }
}

impl std::fmt::Debug for MemoizeEqCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoizeCache").finish()
    }
}

/// A `HashMap` tagged by a phantom marker type `T` so that maps for different markers are
/// distinct types in `anymap3`. Two memoized functions with identical `K`/`V` types stay in
/// separate entries because their `T` (the function type) differs.
struct MarkedHashMap<T, K, V> {
    inner: HashMap<K, V, RandomState>,
    _type: PhantomData<fn() -> T>,
}

impl<T, K, V> MarkedHashMap<T, K, V> {
    fn new() -> Self {
        Self {
            inner: HashMap::with_hasher(RandomState::new()),
            _type: PhantomData,
        }
    }
}

impl<T, K, V> Deref for MarkedHashMap<T, K, V> {
    type Target = HashMap<K, V, RandomState>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, K, V> DerefMut for MarkedHashMap<T, K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// A newtype wrapper around the argument tuple. It exists so we can implement `Equivalent` and
/// `ToOwnedKey` for tuples of references against the corresponding tuple of owned values, which
/// would otherwise run into orphan rules and conflicting blanket impls.
#[doc(hidden)]
#[derive(Hash)]
pub struct MemoizeKey<T>(T);

/// Converts a borrowed key (e.g. `(&str, &i32)`) into the owned key stored in the map
/// (e.g. `(String, i32)`). Used only on cache misses, when we need to insert.
#[doc(hidden)]
pub trait ToOwnedKey {
    type Owned;
    fn to_owned_key(&self) -> Self::Owned;
}

/// Generates `Equivalent` and `ToOwnedKey` impls for argument tuples up to arity 12, so callers
/// can pass keys made of borrowed values and still hit entries stored as owned values.
macro_rules! impl_tuple {
    ($(($kty:ident, $qty:ident, $accessor:tt)),*) => {
        impl<$($kty, $qty),*> Equivalent<($($kty,)*)> for MemoizeKey<($(&$qty,)*)>
        where
            $(
                $qty: ?Sized + Equivalent<$kty>,
            )*
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

    // Hand-written zero-arity impls for memoized functions whose only parameter is `cx`. The
    // macro's `&&*`-joined body doesn't expand cleanly for zero repetitions.
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
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Returns a fresh counter with `'static` lifetime so closures that capture it can be
    /// `Copy + 'static` (the bounds `MemoizeCache::memoize` imposes on its function).
    fn counter() -> &'static AtomicUsize {
        Box::leak(Box::new(AtomicUsize::new(0)))
    }

    #[test]
    fn sync_same_key_runs_body_once() {
        let cache = MemoizeEqCache::new();
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
        let cache = MemoizeEqCache::new();
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
        let cache = MemoizeEqCache::new();
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
        let cache = MemoizeEqCache::new();
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
    fn sync_zero_arity_key() {
        let cache = MemoizeEqCache::new();
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

    #[tokio::test]
    async fn async_same_key_runs_body_once() {
        let cache = MemoizeEqCache::new();
        let cx = Cx::default();
        let n = counter();
        let f = async move |_: &Cx, (x, y): (i32, i32)| {
            n.fetch_add(1, Ordering::SeqCst);
            x + y
        };

        let a = cache.memoize_async(&cx, (&1i32, &2i32), (1, 2), f).await;
        let b = cache.memoize_async(&cx, (&1i32, &2i32), (1, 2), f).await;

        assert_eq!(*a, 3);
        assert_eq!(*b, 3);
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn async_different_keys_run_body_per_key() {
        let cache = MemoizeEqCache::new();
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
}
