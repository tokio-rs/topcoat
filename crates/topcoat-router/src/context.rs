use axum::{body::Body, extract::Request};
use tokio::task_local;

#[derive(Debug, Clone, Copy)]
pub struct Cx<'a> {
    request: &'a Request<Body>,
}

task_local! {
    static CX: Cx<'static>;
}

pub(crate) async fn with_context<F: Future>(request: &Request<Body>, f: F) -> F::Output {
    // SAFETY: `CX` requires a `'static` type, but the value is only accessible
    // for the duration of `scope`, which awaits `f` to completion before
    // returning. The borrow therefore cannot outlive `request`.
    let cx: Cx<'static> = unsafe { std::mem::transmute(Cx { request }) };
    CX.scope(cx, f).await
}
