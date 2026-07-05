use http::header::LOCATION;
use http::{HeaderValue, StatusCode};
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{IntoResponse, Response};

/// Builds a temporary (HTTP 307) redirect to `uri`.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # async fn lookup(_cx: &Cx, _id: u64) -> Option<User> { None }
/// use topcoat::Result;
/// use topcoat::context::Cx;
/// use topcoat::router::redirect;
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let Some(user) = lookup(cx, id).await else {
///         return Err(redirect("/users").into());
///     };
///     Ok(user)
/// }
/// ```
#[must_use]
pub fn redirect(uri: &str) -> RedirectError {
    RedirectError::new(StatusCode::TEMPORARY_REDIRECT, uri)
}

/// Builds a permanent (HTTP 308) redirect to `uri`.
///
/// Use this for URLs that have moved for good; clients and search engines
/// are allowed to cache the new location.
///
/// # Examples
///
/// ```rust
/// use topcoat::Result;
/// use topcoat::context::Cx;
/// use topcoat::router::{page, redirect_permanent};
///
/// #[page]
/// async fn legacy_profile(cx: &Cx) -> Result {
///     Err(redirect_permanent("/profile").into())
/// }
/// ```
#[must_use]
pub fn redirect_permanent(uri: &str) -> RedirectError {
    RedirectError::new(StatusCode::PERMANENT_REDIRECT, uri)
}

/// A redirect response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`redirect`] or [`redirect_permanent`], or derive one
/// from an `Option` / `Result` via [`RouterErrorExt`](crate::runtime::RouterErrorExt).
#[derive(Debug)]
pub struct RedirectError {
    status: StatusCode,
    location: HeaderValue,
}

impl RedirectError {
    /// Builds a redirect with the given status code and target `uri`.
    ///
    /// # Panics
    ///
    /// Panics if `uri` is not a valid `Location` header value.
    fn new(status: StatusCode, uri: &str) -> Self {
        Self {
            status,
            location: HeaderValue::try_from(uri).expect("redirect uri is not a valid header value"),
        }
    }
}

impl std::fmt::Display for RedirectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("redirect")
    }
}

impl std::error::Error for RedirectError {}

impl IntoResponse for RedirectError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (self.status, ([(LOCATION, self.location)], ())).into_response(cx)
    }
}
