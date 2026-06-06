use axum::extract::RawPathParams;
use http::request::Parts;
use topcoat_core::runtime::context::{Cx, app_state, request_state};

/// Returns the [`Parts`] of the current request.
///
/// Use this when you need access to multiple components of the request at
/// once. For individual fields, prefer the dedicated accessors
/// ([`method`], [`uri`], [`version`], [`headers`], [`extensions`]).
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{context::Cx, router::parts};
///
/// async fn log_request(cx: &Cx) {
///     let parts = parts(cx);
///     println!("{} {}", parts.method, parts.uri);
/// }
/// ```
#[inline]
#[must_use]
pub fn parts(cx: &Cx) -> &Parts {
    request_state(cx)
}

/// Returns the HTTP [`Method`] of the current request.
///
/// [`Method`]: http::Method
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{context::Cx, router::method};
///
/// async fn is_post(cx: &Cx) -> bool {
///     method(cx) == http::Method::POST
/// }
/// ```
#[inline]
#[must_use]
pub fn method(cx: &Cx) -> &http::Method {
    &parts(cx).method
}

/// Returns the [`Uri`] of the current request.
///
/// [`Uri`]: http::Uri
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{context::Cx, router::uri};
///
/// async fn current_path(cx: &Cx) -> &str {
///     uri(cx).path()
/// }
/// ```
#[inline]
#[must_use]
pub fn uri(cx: &Cx) -> &http::Uri {
    &parts(cx).uri
}

/// Returns the HTTP [`Version`] of the current request.
///
/// [`Version`]: http::Version
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{context::Cx, router::version};
///
/// async fn is_http2(cx: &Cx) -> bool {
///     *version(cx) == http::Version::HTTP_2
/// }
/// ```
#[inline]
#[must_use]
pub fn version(cx: &Cx) -> &http::Version {
    &parts(cx).version
}

/// Returns the [`HeaderMap`] of the current request.
///
/// [`HeaderMap`]: http::HeaderMap
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{context::Cx, router::headers};
///
/// async fn user_agent(cx: &Cx) -> Option<&str> {
///     headers(cx).get("user-agent")?.to_str().ok()
/// }
/// ```
#[inline]
#[must_use]
pub fn headers(cx: &Cx) -> &http::HeaderMap {
    &parts(cx).headers
}

/// Returns the [`Extensions`] of the current request.
///
/// Extensions carry typed values attached to the request, typically by
/// middleware running before the handler.
///
/// [`Extensions`]: http::Extensions
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{context::Cx, router::extensions};
///
/// struct RequestId(String);
///
/// async fn request_id(cx: &Cx) -> Option<&str> {
///     extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
/// }
/// ```
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

/// Runs an [`axum`] extractor against the current request.
///
/// Prefer the dedicated `topcoat` accessors ([`parts`], [`method`], [`uri`],
/// [`version`], [`headers`], [`extensions`], and others whenever possible.
/// This function exists as an escape hatch for interoperability with
/// third-party libraries that only expose their API through
/// [`FromRequestParts`], or for porting existing `axum` code.
///
/// [`FromRequestParts`]: axum::extract::FromRequestParts
///
/// # Examples
///
/// ```rust,ignore
/// use axum::extract::Query;
/// use serde::Deserialize;
/// use topcoat::{context::Cx, router::extract};
///
/// #[derive(Deserialize)]
/// struct Pagination {
///     page: usize,
/// }
///
/// async fn page(cx: &Cx) -> usize {
///     let Query(p): Query<Pagination> = extract::<_, ()>(cx).await.unwrap();
///     p.page
/// }
/// ```
pub async fn extract<T, S>(
    cx: &Cx,
) -> Result<T, <T as axum::extract::FromRequestParts<S>>::Rejection>
where
    T: axum::extract::FromRequestParts<S>,
    S: Send + Sync + 'static,
{
    T::from_request_parts(&mut parts(cx).clone(), app_state::<S>(cx)).await
}
