use http::StatusCode;

use crate::Response;

/// Builds a forbidden (HTTP 403) response.
///
/// Use this when the caller is authenticated but not permitted to access
/// the resource.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::Result;
/// use topcoat::router::forbidden;
///
/// async fn admin_panel(cx: &Cx, user: &User) -> Result<View> {
///     if !user.is_admin() {
///         return Err(forbidden().into());
///     }
///     Ok(render_admin(cx))
/// }
/// ```
pub fn forbidden() -> ForbiddenError {
    ForbiddenError::new()
}

/// A forbidden response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`forbidden`], or derive one from an `Option` /
/// `Result` via [`crate::RouterErrorExt`].
#[derive(Debug)]
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

impl axum::response::IntoResponse for ForbiddenError {
    fn into_response(self) -> Response {
        (StatusCode::FORBIDDEN, "forbidden").into_response()
    }
}
