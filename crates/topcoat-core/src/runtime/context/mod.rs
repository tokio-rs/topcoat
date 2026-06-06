mod abort;
mod memoize;
mod state;

pub use abort::*;
pub use memoize::*;
pub use state::*;

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
    app_state: Arc<State>,
    request_state: State,
    cache: MemoizeCache,
    abort: AbortStore,
}

impl Cx {
    pub fn new(app_state: Arc<State>, request_state: State) -> Self {
        Self {
            id: CxId::new(),
            app_state,
            request_state,
            cache: MemoizeCache::new(),
            abort: AbortStore::new(),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        Self::new(Arc::new(State::new()), State::new())
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
}

impl Default for Cx {
    fn default() -> Self {
        Cx::new(Arc::new(State::default()), State::default())
    }
}
