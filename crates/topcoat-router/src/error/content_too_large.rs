use http::StatusCode;
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Creates a `413 Content Too Large` error.
///
/// The built-in extractors return this error when a request body is longer
/// than the request's body limit. See [`BodyLimit`](crate::BodyLimit) to
/// change that limit. Return it yourself when the input is too large by some
/// other measure.
///
/// # Examples
///
/// ```rust
/// use topcoat::{Result, router::error::content_too_large};
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

/// A `413 Content Too Large` error.
///
/// Create one with [`content_too_large`]. Returned from a handler, it renders
/// as a `413 Content Too Large` response.
#[derive(Debug, Clone)]
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
