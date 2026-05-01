use std::{
    any::{Any, TypeId},
    collections::hash_map::RandomState,
    future::Future,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex, OnceLock},
};

use hashbrown::{Equivalent, HashMap};
use tokio::sync::OnceCell;

#[derive(Debug, Clone)]
pub struct Memoized<'a, T> {
    inner: Arc<T>,
    // We artificially limit the lifetime of a memoized value to be the lifetime of the request
    // context. This is because the `Arc` is an implementation detail of the cache. The user should
    // not be able to hold on to the memoized value as long as they want. Conceptually, the cache
    // only lasts as long as the request context. The implementation might change to be more
    // efficient in the future.
    lifetime: PhantomData<&'a ()>,
}

impl<'a, T> Memoized<'a, T> {
    fn new(inner: Arc<T>) -> Self {
        Self {
            inner,
            lifetime: PhantomData,
        }
    }
}

impl<'a, T> Deref for Memoized<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[doc(hidden)]
pub struct MemoizeCache {
    entries: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl MemoizeCache {
    pub(super) fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn memoize<'a, Q, K, V, F>(&'a self, borrowed_key: Q, key: K, f: F) -> Memoized<'a, V>
    where
        Q: Copy,
        MemoizeKey<Q>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<Q> as ToOwnedKey>::Owned>,
        <MemoizeKey<Q> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: (FnOnce(K) -> V) + 'static,
    {
        let cell = {
            let mut guard = self.entries.lock().unwrap();
            let cache = guard.entry(TypeId::of::<F>()).or_insert_with(|| {
                Box::new(HashMap::<
                    <MemoizeKey<Q> as ToOwnedKey>::Owned,
                    Arc<OnceLock<Arc<V>>>,
                    RandomState,
                >::with_hasher(RandomState::new()))
            });
            let cache =
                cache
                    .downcast_mut::<HashMap<
                        <MemoizeKey<Q> as ToOwnedKey>::Owned,
                        Arc<OnceLock<Arc<V>>>,
                        RandomState,
                    >>()
                    .unwrap();

            if let Some(cell) = cache.get(&MemoizeKey(borrowed_key)) {
                cell.clone()
            } else {
                let cell = Arc::new(OnceLock::new());
                let key_owned = MemoizeKey(borrowed_key).to_owned_key();
                cache.insert(key_owned, cell.clone());
                cell
            }
        };

        let value = cell.get_or_init(|| Arc::new(f(key)));
        Memoized::new(value.clone())
    }

    pub async fn memoize_async<'a, Q, K, V, F, Fut>(
        &'a self,
        borrowed_key: Q,
        key: K,
        f: F,
    ) -> Memoized<'a, V>
    where
        Q: Copy,
        MemoizeKey<Q>: Hash + ToOwnedKey + Equivalent<<MemoizeKey<Q> as ToOwnedKey>::Owned>,
        <MemoizeKey<Q> as ToOwnedKey>::Owned: Hash + Eq + Send + Sync + 'static,
        V: Send + Sync + 'static,
        F: (FnOnce(K) -> Fut) + 'static,
        Fut: Future<Output = V>,
    {
        let cell = {
            let mut guard = self.entries.lock().unwrap();
            let cache = guard.entry(TypeId::of::<F>()).or_insert_with(|| {
                Box::new(HashMap::<
                    <MemoizeKey<Q> as ToOwnedKey>::Owned,
                    Arc<OnceCell<Arc<V>>>,
                    RandomState,
                >::with_hasher(RandomState::new()))
            });
            let cache =
                cache
                    .downcast_mut::<HashMap<
                        <MemoizeKey<Q> as ToOwnedKey>::Owned,
                        Arc<OnceCell<Arc<V>>>,
                        RandomState,
                    >>()
                    .unwrap();

            if let Some(cell) = cache.get(&MemoizeKey(borrowed_key)) {
                cell.clone()
            } else {
                let cell = Arc::new(OnceCell::new());
                let key_owned = MemoizeKey(borrowed_key).to_owned_key();
                cache.insert(key_owned, cell.clone());
                cell
            }
        };

        let value = cell.get_or_init(|| async { Arc::new(f(key).await) }).await;
        Memoized::new(value.clone())
    }
}

impl std::fmt::Debug for MemoizeCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoizeCache").finish()
    }
}

#[derive(Hash)]
pub struct MemoizeKey<T>(T);

pub trait ToOwnedKey {
    type Owned;
    fn to_owned_key(&self) -> Self::Owned;
}

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
