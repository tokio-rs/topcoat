use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

use tokio::sync::OnceCell;

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

    pub fn memoize<'a, K, V, F>(&'a self, key: K, f: F) -> Memoized<'a, V>
    where
        K: MemoizeKey,
        <K as MemoizeKey>::Owned: Borrow<K>,
        V: Send + Sync + 'static,
        F: (FnOnce(K) -> V) + 'static,
    {
        let mut guard = self.entries.lock().unwrap();
        let cache = guard
            .entry(TypeId::of::<F>())
            .or_insert_with(|| Box::new(HashMap::<<K as MemoizeKey>::Owned, Arc<V>>::new()));
        let cache = cache
            .downcast_mut::<HashMap<<K as MemoizeKey>::Owned, Arc<V>>>()
            .unwrap();

        if let Some(value) = cache.get(&key) {
            Memoized::new(value.clone())
        } else {
            let key_owned = key.to_owned_key();
            let value = Arc::new(f(key));
            cache.insert(key_owned, value.clone());
            Memoized::new(value)
        }
    }

    pub async fn memoize_async<'a, K, V, F, Fut>(&'a self, key: K, f: F) -> Memoized<'a, V>
    where
        K: MemoizeKey,
        <K as MemoizeKey>::Owned: Borrow<K>,
        V: Send + Sync + 'static,
        F: (FnOnce(K) -> Fut) + 'static,
        Fut: Future<Output = V>,
    {
        let cell = {
            let mut guard = self.entries.lock().unwrap();
            let cache = guard.entry(TypeId::of::<F>()).or_insert_with(|| {
                Box::new(HashMap::<<K as MemoizeKey>::Owned, Arc<OnceCell<Arc<V>>>>::new())
            });
            let cache = cache
                .downcast_mut::<HashMap<<K as MemoizeKey>::Owned, Arc<OnceCell<Arc<V>>>>>()
                .unwrap();

            if let Some(cell) = cache.get(&key) {
                cell.clone()
            } else {
                let cell = Arc::new(OnceCell::new());
                let key_owned = key.to_owned_key();
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

pub trait MemoizeKey: Eq + Hash {
    type Owned: Eq + Hash + Send + Sync + 'static;

    fn to_owned_key(&self) -> Self::Owned;
}

macro_rules! impl_memoize_key_tuple {
    ($(($ty:ident, $accessor:tt)),*) => {
        impl<$($ty),*> crate::context::MemoizeKey for ($($ty,)*)
        where
            $(
                $ty: ToOwned + Eq + std::hash::Hash,
                <$ty as ToOwned>::Owned: Eq + std::hash::Hash + Send + Sync + 'static,
            )*
        {
            type Owned = ($($ty::Owned,)*);

            fn to_owned_key(&self) -> Self::Owned {
                ($(self.$accessor.to_owned(),)*)
            }
        }
    };
}

#[rustfmt::skip]
mod impls {
    impl_memoize_key_tuple!((T1, 0));
    impl_memoize_key_tuple!((T1, 0), (T2, 1));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9));
    impl_memoize_key_tuple!((T0, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10));
    impl_memoize_key_tuple!((T1, 0), (T2, 1), (T3, 2), (T4, 3), (T5, 4), (T6, 5), (T7, 6), (T8, 7), (T9, 8), (T10, 9), (T11, 10), (T12, 11));
}
