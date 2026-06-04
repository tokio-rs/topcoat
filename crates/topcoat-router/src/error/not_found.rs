use http::StatusCode;

use crate::Response;

/// Builds a not-found (HTTP 404) response.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::Result;
/// use topcoat::router::not_found;
///
/// async fn fetch_user(cx: &Cx, id: u64) -> Result<User> {
///     let Some(user) = lookup(cx, id).await else {
///         return Err(not_found().into());
///     };
///     Ok(user)
/// }
/// ```
pub fn not_found() -> NotFoundError {
    NotFoundError::new()
}

/// A not-found response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`not_found`], or derive one from an `Option` /
/// `Result` via [`crate::RouterErrorExt`].
#[derive(Debug)]
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

impl axum::response::IntoResponse for NotFoundError {
    fn into_response(self) -> Response {
        (StatusCode::NOT_FOUND, "not found").into_response()
    }
}
