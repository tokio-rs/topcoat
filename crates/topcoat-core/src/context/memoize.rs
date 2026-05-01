use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    ops::Deref,
    sync::{Arc, Mutex},
};

use tokio::sync::OnceCell;

use crate::context::Cx;

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

pub(super) struct MemoizeCache {
    entries: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,
}

impl MemoizeCache {
    pub(super) fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }
}

impl std::fmt::Debug for MemoizeCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoizeCache").finish()
    }
}

pub fn memoize_raw<'a, K, V, F>(cx: &Cx, type_id: TypeId, key: K, f: F) -> Memoized<'a, V>
where
    K: ToOwned + Eq + Hash,
    <K as ToOwned>::Owned: Eq + Hash + Send + Sync + 'static,
    V: Send + Sync + 'static,
    F: FnOnce(K) -> V,
{
    let mut guard = cx.cache.entries.lock().unwrap();
    let cache = guard
        .entry(type_id)
        .or_insert_with(|| Box::new(HashMap::<<K as ToOwned>::Owned, Arc<V>>::new()));
    let cache = cache
        .downcast_mut::<HashMap<<K as ToOwned>::Owned, Arc<V>>>()
        .unwrap();

    if let Some(value) = cache.get(&key) {
        Memoized::new(value.clone())
    } else {
        let key_owned = key.to_owned();
        let value = Arc::new(f(key));
        cache.insert(key_owned, value.clone());
        Memoized::new(value)
    }
}

pub async fn memoize_raw_async<'a, K, V, F, Fut>(
    cx: &Cx,
    type_id: TypeId,
    key: K,
    f: F,
) -> Memoized<'a, V>
where
    K: ToOwned + Eq + Hash,
    <K as ToOwned>::Owned: Eq + Hash + Send + Sync + 'static,
    V: Send + Sync + 'static,
    F: FnOnce(K) -> Fut,
    Fut: Future<Output = V>,
{
    let cell = {
        let mut guard = cx.cache.entries.lock().unwrap();
        let cache = guard.entry(type_id).or_insert_with(|| {
            Box::new(HashMap::<<K as ToOwned>::Owned, Arc<OnceCell<Arc<V>>>>::new())
        });
        let cache = cache
            .downcast_mut::<HashMap<<K as ToOwned>::Owned, Arc<OnceCell<Arc<V>>>>>()
            .unwrap();

        if let Some(cell) = cache.get(&key) {
            cell.clone()
        } else {
            let cell = Arc::new(OnceCell::new());
            let key_owned = key.to_owned();
            cache.insert(key_owned, cell.clone());
            cell
        }
    };

    let value = cell.get_or_init(|| async { Arc::new(f(key).await) }).await;
    Memoized::new(value.clone())
}
