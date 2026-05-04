//! Error types returned from page and layout handlers.

use axum::response::IntoResponse;
use http::StatusCode;

use crate::RedirectError;

/// The result type returned from page and layout handlers.
///
/// Defaults to `Result<View, Error>` so handlers can be written as
/// `-> Result` when they produce a [`View`](topcoat_view::runtime::View) on
/// success and an [`Error`] on failure.
pub type Result<T = topcoat_view::runtime::View, E = Error> = core::result::Result<T, E>;

/// A non-success outcome from a handler.
#[derive(Debug)]
pub enum Error {
    /// A redirect short-circuiting the request to another URL.
    Redirect(RedirectError),
    /// An unexpected failure.
    InternalServer(InternalServerError),
}

impl From<RedirectError> for Error {
    fn from(value: RedirectError) -> Self {
        Self::Redirect(value)
    }
}

/// An unexpected failure raised from a handler.
///
/// The wrapped error is captured for logging but never exposed to the client.
#[derive(Debug)]
pub struct InternalServerError {
    _inner: Box<dyn std::error::Error + Send + Sync>,
}

impl From<InternalServerError> for Error {
    fn from(value: InternalServerError) -> Self {
        Self::InternalServer(value)
    }
}

impl IntoResponse for InternalServerError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, "internal sever error").into_response()
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Redirect(inner) => inner.into_response(),
            Self::InternalServer(inner) => inner.into_response(),
        }
    }
}
