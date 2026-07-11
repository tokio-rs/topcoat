mod eq_cache;

pub use eq_cache::*;

/// All memoize cache variants grouped into one structure.
#[derive(Debug, Default)]
#[doc(hidden)]
pub struct MemoizeCache {
    eq_cache: MemoizeEqCache,
}

impl MemoizeCache {
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        MemoizeCache::default()
    }

    #[inline]
    pub fn eq_cache(&self) -> &MemoizeEqCache {
        &self.eq_cache
    }
}
