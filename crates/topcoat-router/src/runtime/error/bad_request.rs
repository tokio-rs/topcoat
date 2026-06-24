use http::StatusCode;
use topcoat_core::runtime::error::Result;

use crate::runtime::{IntoResponse, Response};

/// Builds a bad-request (HTTP 400) response with a client-safe description.
///
/// Use this when the caller supplied invalid input and the response should
/// explain what was wrong.
///
/// # Examples
///
/// ```rust
/// use topcoat::Result;
/// use topcoat::router::bad_request;
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

/// Builds a bad-request (HTTP 400) response whose description includes an
/// input path.
///
/// This is useful for structured request formats where the parser can report
/// the field or element that failed validation.
pub fn bad_request_at(
    path: impl std::fmt::Display,
    description: impl Into<String>,
) -> BadRequestError {
    let path = path.to_string();
    let description = description.into();
    BadRequestError::new(Some(path), description)
}

/// A bad-request response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`bad_request`].
#[derive(Debug)]
pub struct BadRequestError {
    path: Option<String>,
    description: String,
}

impl BadRequestError {
    fn new(path: Option<String>, description: String) -> Self {
        Self { path, description }
    }

    /// Returns the path into the request where the error was encountered.
    #[must_use]
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Returns the client-safe description of what was wrong with the request.
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
    fn into_response(self) -> Result<Response> {
        (StatusCode::BAD_REQUEST, self.to_string()).into_response()
    }
}
