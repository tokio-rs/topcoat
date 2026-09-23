use http::StatusCode;
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Creates a `404 Not Found` error.
///
/// Use it when the requested resource does not exist. The router returns this
/// error on its own when no route matches the request path.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # async fn lookup(_cx: &Cx, _id: u64) -> Option<User> { None }
/// use topcoat::{Result, context::Cx, router::error::not_found};
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let Some(user) = lookup(cx, id).await else {
///         return Err(not_found().into());
///     };
///     Ok(user)
/// }
/// ```
#[must_use]
pub fn not_found() -> NotFoundError {
    NotFoundError::new()
}

/// A `404 Not Found` error.
///
/// Create one with [`not_found`], or turn a missing value into one with
/// [`RouterErrorExt`](crate::error::RouterErrorExt). Returned from a handler,
/// it renders as a `404 Not Found` response.
#[derive(Debug, Clone)]
pub struct NotFoundError {
    _priv: (),
}

impl NotFoundError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl std::fmt::Display for NotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not found")
    }
}

impl std::error::Error for NotFoundError {}

impl IntoResponse for NotFoundError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::NOT_FOUND, "not found").into_response(cx)
    }
}
