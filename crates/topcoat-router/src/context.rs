use std::sync::Arc;

use axum::extract::RawPathParams;
use http::request::Parts;
use tokio::task_local;

#[derive(Debug)]
pub struct Cx {
    parts: Parts,
    params: RawPathParams,
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

pub(crate) async fn scope_context<F: Future>(
    parts: Parts,
    params: RawPathParams,
    f: F,
) -> F::Output {
    CX.scope(Arc::new(Cx { parts, params }), f).await
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

#[inline]
#[must_use]
pub fn parts(cx: &Cx) -> &Parts {
    &cx.parts
}

#[inline]
#[must_use]
pub fn method(cx: &Cx) -> &http::Method {
    &parts(cx).method
}

#[inline]
#[must_use]
pub fn uri(cx: &Cx) -> &http::Uri {
    &parts(cx).uri
}

#[inline]
#[must_use]
pub fn version(cx: &Cx) -> &http::Version {
    &parts(cx).version
}

#[inline]
#[must_use]
pub fn headers(cx: &Cx) -> &http::HeaderMap {
    &parts(cx).headers
}

#[inline]
#[must_use]
pub fn extensions(cx: &Cx) -> &http::Extensions {
    &parts(cx).extensions
}

/// This is an internal function, use direct path hooks instead.
#[inline]
#[must_use]
#[doc(hidden)]
pub fn raw_path_params(cx: &Cx) -> &RawPathParams {
    &cx.params
}
