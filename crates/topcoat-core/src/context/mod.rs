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

    pub fn id(&self) -> CxId {
        self.id
    }

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
    app_state: Arc<State>,
    request_state: State,
    f: F,
) -> MaybeAborted<F::Output> {
    let cx = Arc::new(Cx::new(app_state, request_state));
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
