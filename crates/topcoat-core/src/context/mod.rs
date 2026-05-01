mod memoize;
mod parts;

pub use memoize::*;
pub use parts::*;

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use axum::extract::RawPathParams;
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

    parts: Parts,
    params: RawPathParams,

    cache: MemoizeCache,
}

impl Cx {
    pub fn id(&self) -> CxId {
        self.id
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

pub async fn scope_context<F: Future>(parts: Parts, params: RawPathParams, f: F) -> F::Output {
    CX.scope(
        Arc::new(Cx {
            id: CxId::new(),
            parts,
            params,
            cache: MemoizeCache::new(),
        }),
        f,
    )
    .await
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
