use http::StatusCode;

use crate::Response;

/// Builds a bad-request (HTTP 400) response with a client-safe description.
///
/// Use this when the caller supplied invalid input and the response should
/// explain what was wrong.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::Result;
/// use topcoat::router::bad_request;
///
/// async fn update_user(name: String) -> Result {
///     if name.trim().is_empty() {
///         return Err(bad_request("name cannot be empty").into());
///     }
///
///     Ok(())
/// }
/// ```
pub fn bad_request(description: impl Into<String>) -> BadRequestError {
    BadRequestError::new(description.into())
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

    if path == "." {
        bad_request(description)
    } else {
        bad_request(format!("{path}: {description}"))
    }
}

/// A bad-request response carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`bad_request`].
#[derive(Debug)]
pub struct BadRequestError {
    description: String,
}

impl BadRequestError {
    fn new(description: String) -> Self {
        Self { description }
    }

    /// Returns the client-safe description of what was wrong with the request.
    pub fn description(&self) -> &str {
        &self.description
    }
}

impl std::fmt::Display for BadRequestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bad request: {}", self.description)
    }
}

impl std::error::Error for BadRequestError {}

impl axum::response::IntoResponse for BadRequestError {
    fn into_response(self) -> Response {
        <(StatusCode, String) as axum::response::IntoResponse>::into_response((
            StatusCode::BAD_REQUEST,
            self.to_string(),
        ))
    }
}
