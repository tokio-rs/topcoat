use http::{HeaderValue, Method, StatusCode};
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Body,
    response::{IntoResponse, Response},
};

/// Creates a `405 Method Not Allowed` error.
///
/// `methods` are the methods the path supports. The response lists them in
/// its `Allow` header. The router returns this error on its own when a
/// request's path matches a route but its method does not.
///
/// # Examples
///
/// ```rust
/// use topcoat::router::{Method, error::method_not_allowed};
///
/// let error = method_not_allowed([Method::GET, Method::POST]);
/// ```
pub fn method_not_allowed(methods: impl IntoIterator<Item = Method>) -> MethodNotAllowedError {
    MethodNotAllowedError::new(methods)
}

/// A `405 Method Not Allowed` error.
///
/// Create one with [`method_not_allowed`]. Returned from a handler, it
/// renders as a `405 Method Not Allowed` response with an `Allow` header.
#[derive(Debug, Clone)]
pub struct MethodNotAllowedError {
    /// The value of the `Allow` header: the supported methods, comma-separated.
    allow: String,
}

impl MethodNotAllowedError {
    fn new(methods: impl IntoIterator<Item = Method>) -> Self {
        let allow = methods
            .into_iter()
            .map(|method| method.as_str().to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        Self { allow }
    }
}

impl std::fmt::Display for MethodNotAllowedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("method not allowed")
    }
}

impl std::error::Error for MethodNotAllowedError {}

impl IntoResponse for MethodNotAllowedError {
    fn into_response(self, _cx: &Cx) -> Result<Response> {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
        if let Ok(allow) = HeaderValue::from_str(&self.allow) {
            response.headers_mut().insert(http::header::ALLOW, allow);
        }
        Ok(response)
    }
}
