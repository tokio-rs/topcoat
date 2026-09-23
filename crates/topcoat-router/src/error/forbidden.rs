use http::StatusCode;
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Creates a `403 Forbidden` error.
///
/// Use it when the client is authenticated but not allowed to access the
/// resource. For a client that is not authenticated, use
/// [`unauthorized`](crate::error::unauthorized).
///
/// # Examples
///
/// ```rust
/// # use topcoat::view::View;
/// # struct User;
/// # impl User { fn is_admin(&self) -> bool { true } }
/// # fn render_admin(_cx: &Cx) -> impl View {}
/// use topcoat::{Result, context::Cx, router::error::forbidden};
///
/// async fn admin_panel(cx: &Cx, user: &User) -> Result<impl View> {
///     if !user.is_admin() {
///         return Err(forbidden().into());
///     }
///     Ok(render_admin(cx))
/// }
/// ```
#[must_use]
pub fn forbidden() -> ForbiddenError {
    ForbiddenError::new()
}

/// A `403 Forbidden` error.
///
/// Create one with [`forbidden`], or turn a missing value into one with
/// [`RouterErrorExt`](crate::error::RouterErrorExt). Returned from a handler,
/// it renders as a `403 Forbidden` response.
#[derive(Debug, Clone)]
pub struct ForbiddenError {
    _priv: (),
}

impl ForbiddenError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl std::fmt::Display for ForbiddenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("forbidden")
    }
}

impl std::error::Error for ForbiddenError {}

impl IntoResponse for ForbiddenError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::FORBIDDEN, "forbidden").into_response(cx)
    }
}
