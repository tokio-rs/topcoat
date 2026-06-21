mod context_map;
mod id;

pub use context_map::*;
pub use id::*;

use std::{any::Any, sync::Arc};

use crate::runtime::{abort::AbortStore, memoize::MemoizeCache};

#[derive(Debug)]
pub struct Cx {
    id: CxId,
    pub(crate) app_context: Arc<ContextMap>,
    pub(crate) request_context: ContextMap,
    memoize_cache: MemoizeCache,
    abort_store: AbortStore,
}

impl Cx {
    pub fn new(app_context: Arc<ContextMap>, request_context: ContextMap) -> Self {
        Self {
            id: CxId::new(),
            app_context,
            request_context,
            memoize_cache: MemoizeCache::new(),
            abort_store: AbortStore::new(),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        Self::new(Arc::new(ContextMap::new()), ContextMap::new())
    }

    pub fn insert<T>(&mut self, value: T)
    where
        T: Any + Send + Sync,
    {
        self.request_context.insert(value);
    }

    #[inline]
    pub fn id(&self) -> CxId {
        self.id
    }
}

impl Default for Cx {
    fn default() -> Self {
        Cx::new(Arc::new(ContextMap::default()), ContextMap::default())
    }
}

#[inline]
#[doc(hidden)]
pub fn memoize_cache(cx: &Cx) -> &MemoizeCache {
    &cx.memoize_cache
}

#[inline]
#[doc(hidden)]
pub fn abort_store(cx: &Cx) -> &AbortStore {
    &cx.abort_store
}
