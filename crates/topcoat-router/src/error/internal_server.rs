use http::StatusCode;
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::response::{IntoResponse, Response};

/// Creates a `500 Internal Server Error` error that wraps `error`.
///
/// The response does not show `error` to the client. Any error that is not a
/// router error already renders as a `500 Internal Server Error`, so use this
/// function when you need the error as an [`InternalServerError`] value.
///
/// # Examples
///
/// ```rust
/// # use topcoat::Error;
/// # struct Dashboard;
/// # async fn fetch_dashboard(_cx: &Cx) -> Result<Dashboard, Error> { Ok(Dashboard) }
/// use topcoat::{Result, context::Cx, router::error::internal_server_error};
///
/// async fn load_dashboard(cx: &Cx) -> Result<Dashboard> {
///     let dashboard = fetch_dashboard(cx).await.map_err(internal_server_error)?;
///
///     Ok(dashboard)
/// }
/// ```
pub fn internal_server_error(error: impl Into<Error>) -> InternalServerError {
    InternalServerError::new(error.into())
}

/// A `500 Internal Server Error` error.
///
/// Create one with [`internal_server_error`] or [`From`]. Returned from a
/// handler, it renders as a `500 Internal Server Error` response that hides
/// the wrapped error from the client.
#[derive(Debug, Clone)]
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

impl IntoResponse for InternalServerError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::INTERNAL_SERVER_ERROR, "internal server error").into_response(cx)
    }
}
