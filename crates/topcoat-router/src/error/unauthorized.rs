use http::StatusCode;
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Creates a `401 Unauthorized` error.
///
/// Use it when the request does not carry valid authentication credentials.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # async fn session(_cx: &Cx) -> Option<User> { None }
/// use topcoat::{Result, context::Cx, router::error::unauthorized};
///
/// async fn current_user(cx: &Cx) -> Result<User> {
///     let Some(user) = session(cx).await else {
///         return Err(unauthorized().into());
///     };
///     Ok(user)
/// }
/// ```
#[must_use]
pub fn unauthorized() -> UnauthorizedError {
    UnauthorizedError::new()
}

/// A `401 Unauthorized` error.
///
/// Create one with [`unauthorized`], or turn a missing value into one with
/// [`RouterErrorExt`](crate::error::RouterErrorExt). Returned from a handler,
/// it renders as a `401 Unauthorized` response.
#[derive(Debug, Clone)]
pub struct UnauthorizedError {
    _priv: (),
}

impl UnauthorizedError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl std::fmt::Display for UnauthorizedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unauthorized")
    }
}

impl std::error::Error for UnauthorizedError {}

impl IntoResponse for UnauthorizedError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response(cx)
    }
}
