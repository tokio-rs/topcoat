use axum::extract::RawPathParams;
use http::request::Parts;
use topcoat_core::context::{Cx, request_state};

#[inline]
#[must_use]
pub fn parts(cx: &Cx) -> &Parts {
    request_state(cx)
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
    request_state::<RawPathParams>(cx)
}
