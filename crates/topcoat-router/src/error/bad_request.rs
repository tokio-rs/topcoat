use http::StatusCode;
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Creates a `400 Bad Request` error with a description of what was wrong.
///
/// Use it when the client sent invalid input. The description is sent to the
/// client in the response body, so it must not contain secrets.
///
/// # Examples
///
/// ```rust
/// use topcoat::{Result, router::error::bad_request};
///
/// async fn update_user(name: String) -> Result<()> {
///     if name.trim().is_empty() {
///         return Err(bad_request("name cannot be empty").into());
///     }
///
///     Ok(())
/// }
/// ```
pub fn bad_request(description: impl Into<String>) -> BadRequestError {
    BadRequestError::new(None, description.into())
}

/// Creates a `400 Bad Request` error that also names the part of the input
/// that was wrong.
///
/// `path` points to the field or element that failed, for example
/// `user.email` in a JSON body. Both the path and the description are sent to
/// the client in the response body.
///
/// # Examples
///
/// ```rust
/// use topcoat::router::error::bad_request_at;
///
/// let error = bad_request_at("user.email", "must contain `@`");
/// assert_eq!(error.path(), Some("user.email"));
/// ```
pub fn bad_request_at(
    path: impl std::fmt::Display,
    description: impl Into<String>,
) -> BadRequestError {
    let path = path.to_string();
    let description = description.into();
    BadRequestError::new(Some(path), description)
}

/// A `400 Bad Request` error.
///
/// Create one with [`bad_request`] or [`bad_request_at`], or turn a missing
/// value into one with [`RouterErrorExt`](crate::error::RouterErrorExt).
/// Returned from a handler, it renders as a `400 Bad Request` response whose
/// body holds the description and, if set, the path.
#[derive(Debug, Clone)]
pub struct BadRequestError {
    path: Option<String>,
    description: String,
}

impl BadRequestError {
    fn new(path: Option<String>, description: String) -> Self {
        Self { path, description }
    }

    /// Returns the path to the part of the input that was wrong, if set.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the description of what was wrong with the request.
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl std::fmt::Display for BadRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(path) => write!(f, "bad request: {} (at `{path}`)", self.description),
            None => write!(f, "bad request: {}", self.description),
        }
    }
}

impl std::error::Error for BadRequestError {}

impl IntoResponse for BadRequestError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response(cx)
    }
}
