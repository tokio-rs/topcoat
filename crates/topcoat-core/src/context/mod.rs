mod abort;
mod app_state;
mod memoize;
mod parts;

pub use abort::*;
pub use app_state::*;
pub use memoize::*;
pub use parts::*;

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use http::request::Parts;
use tokio::task_local;

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
    state: Arc<AppState>,
    parts: Parts,
    cache: MemoizeCache,
    abort: AbortStore,
}

impl Cx {
    pub fn id(&self) -> CxId {
        self.id
    }

    #[doc(hidden)]
    pub fn cache(&self) -> &MemoizeCache {
        &self.cache
    }
}

// `Cx` lives for the entire `scope_context` future, so conceptually we'd just
// store it directly and hand out `&Cx` borrows tied to that scope. We can't:
// `LocalKey::with` only lends `&T` for the duration of its closure (it borrows
// a `RefCell` internally), and the `FnOnce(&T) -> R` bound desugars to a HRTB
// where `R` can't depend on the borrow's lifetime. That makes it impossible to
// return a future that borrows from `cx`. Wrapping in `Arc` sidesteps this:
// we clone the handle out, then borrow from the clone, which lives as long as
// the caller needs.
task_local! {
    static CX: Arc<Cx>;
}

pub async fn scope_context<F: Future>(
    state: Arc<AppState>,
    parts: Parts,
    f: F,
) -> MaybeAborted<F::Output> {
    let cx = Arc::new(Cx {
        id: CxId::new(),
        state,
        parts,
        cache: MemoizeCache::new(),
        abort: AbortStore::new(),
    });
    WatchAbort::new(&cx.clone(), CX.scope(cx, f)).await
}

// `AsyncFnOnce` (rather than `FnOnce`) is required so the returned future is
// allowed to borrow from `&Cx` — see the note on `CX` above.
pub async fn with_context<F, R>(f: F) -> R
where
    F: AsyncFnOnce(&Cx) -> R,
{
    let cx = CX.with(Arc::clone);
    f(&cx).await
}
