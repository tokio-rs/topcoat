use http::StatusCode;
use topcoat_core::{context::Cx, error::Result};

use crate::{IntoResponse, Response};

/// Builds a content-too-large (HTTP 413) response.
///
/// The router raises this itself when a request body exceeds the request's
/// body limit; see [`BodyLimit`](crate::BodyLimit) for changing that limit.
/// Return it yourself when input is too large by a measure of your own.
///
/// # Examples
///
/// ```rust
/// use topcoat::Result;
/// use topcoat::router::error::content_too_large;
///
/// const MAX_COMMENT_CHARS: usize = 4096;
///
/// async fn store_comment(text: String) -> Result<()> {
///     if text.chars().count() > MAX_COMMENT_CHARS {
///         return Err(content_too_large().into());
///     }
///
///     Ok(())
/// }
/// ```
#[must_use]
pub fn content_too_large() -> ContentTooLargeError {
    ContentTooLargeError::new()
}

/// A content-too-large response carried as the `Err` variant of a handler
/// `Result`.
///
/// Construct one with [`content_too_large`].
#[derive(Debug)]
pub struct ContentTooLargeError {
    _priv: (),
}

impl ContentTooLargeError {
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl std::fmt::Display for ContentTooLargeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("content too large")
    }
}

impl std::error::Error for ContentTooLargeError {}

impl IntoResponse for ContentTooLargeError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::PAYLOAD_TOO_LARGE, "content too large").into_response(cx)
    }
}
