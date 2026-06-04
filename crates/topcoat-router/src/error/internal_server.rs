use http::StatusCode;
use topcoat_core::error::Error;

use crate::Response;

/// Builds an internal-server-error (HTTP 500) response.
///
/// Use this when a handler needs to turn an unexpected application error
/// into a response without exposing the underlying error to the client.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::Result;
/// use topcoat::router::internal_server_error;
///
/// async fn load_dashboard(cx: &Cx) -> Result<Dashboard> {
///     let dashboard = fetch_dashboard(cx)
///         .await
///         .map_err(internal_server_error)?;
///
///     Ok(dashboard)
/// }
/// ```
pub fn internal_server_error(error: impl Into<Error>) -> InternalServerError {
    InternalServerError::new(error.into())
}

/// An internal-server-error response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`internal_server_error`].
#[derive(Debug)]
pub struct InternalServerError {
    _inner: Error,
}

impl InternalServerError {
    fn new(inner: Error) -> Self {
        Self { _inner: inner }
    }
}

impl From<Error> for InternalServerError {
    fn from(value: Error) -> Self {
        Self::new(value)
    }
}

impl std::fmt::Display for InternalServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("internal server error")
    }
}

impl std::error::Error for InternalServerError {}

impl axum::response::IntoResponse for InternalServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response()
    }
}
