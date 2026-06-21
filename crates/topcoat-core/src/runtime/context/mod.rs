mod abort;
mod context_map;
mod memoize;

pub use abort::*;
pub use context_map::*;
pub use memoize::*;

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CxId(u64);

impl CxId {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug)]
pub struct Cx {
    id: CxId,
    app_context: Arc<ContextMap>,
    request_context: ContextMap,
    cache: MemoizeCache,
    abort_store: AbortStore,
}

impl Cx {
    pub fn new(app_context: Arc<ContextMap>, request_context: ContextMap) -> Self {
        Self {
            id: CxId::new(),
            app_context,
            request_context,
            cache: MemoizeCache::new(),
            abort_store: AbortStore::new(),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        Self::new(Arc::new(ContextMap::new()), ContextMap::new())
    }

    #[inline]
    pub fn id(&self) -> CxId {
        self.id
    }

    #[inline]
    #[doc(hidden)]
    pub fn cache(&self) -> &MemoizeCache {
        &self.cache
    }

    #[inline]
    #[doc(hidden)]
    pub fn abort_store(&self) -> &AbortStore {
        &self.abort_store
    }
}

impl Default for Cx {
    fn default() -> Self {
        Cx::new(Arc::new(ContextMap::default()), ContextMap::default())
    }
}
