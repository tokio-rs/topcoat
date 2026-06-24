use http::StatusCode;
use topcoat_core::runtime::error::Result;

use crate::runtime::{IntoResponse, Response};

/// Builds an unauthorized (HTTP 401) response.
///
/// Use this when the request lacks valid authentication credentials.
///
/// # Examples
///
/// ```rust
/// # struct User;
/// # async fn session(_cx: &Cx) -> Option<User> { None }
/// use topcoat::Result;
/// use topcoat::context::Cx;
/// use topcoat::router::unauthorized;
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

/// An unauthorized response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`unauthorized`], or derive one from an `Option` /
/// `Result` via [`RouterErrorExt`](crate::runtime::RouterErrorExt).
#[derive(Debug)]
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
    fn into_response(self) -> Result<Response> {
        (StatusCode::UNAUTHORIZED, "unauthorized").into_response()
    }
}
