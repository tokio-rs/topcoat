use http::{HeaderValue, StatusCode, header::RETRY_AFTER};
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Creates a `429 Too Many Requests` error with a `Retry-After` header of
/// `retry_after_secs` seconds.
///
/// Return it when a client went over a limit you set for it, such as a rate
/// limit or a quota. A 429 means that this client sent too many requests. Use
/// [`service_unavailable`](crate::error::service_unavailable) instead when the
/// whole server is busy, no matter who sends the request.
///
/// The `Retry-After` header tells the client when it may try again.
///
/// # Examples
///
/// ```rust
/// use topcoat::{Result, router::error::too_many_requests};
/// # fn tokens_left() -> u32 { 0 }
///
/// async fn handle() -> Result<&'static str> {
///     if tokens_left() == 0 {
///         return Err(too_many_requests(60).into());
///     }
///
///     Ok("served")
/// }
/// ```
#[must_use]
pub fn too_many_requests(retry_after_secs: u64) -> TooManyRequestsError {
    TooManyRequestsError::new(retry_after_secs)
}

/// A `429 Too Many Requests` error.
///
/// Create one with [`too_many_requests`]. Returned from a handler, it renders
/// as a `429 Too Many Requests` response with a `Retry-After` header.
#[derive(Debug, Clone)]
pub struct TooManyRequestsError {
    retry_after_secs: u64,
}

impl TooManyRequestsError {
    fn new(retry_after_secs: u64) -> Self {
        Self { retry_after_secs }
    }

    /// Returns the `Retry-After` value of the response, in seconds.
    #[must_use]
    pub fn retry_after_secs(&self) -> u64 {
        self.retry_after_secs
    }
}

impl std::fmt::Display for TooManyRequestsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "too many requests (retry after {}s)",
            self.retry_after_secs
        )
    }
}

impl std::error::Error for TooManyRequestsError {}

impl IntoResponse for TooManyRequestsError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let mut response =
            (StatusCode::TOO_MANY_REQUESTS, "too many requests").into_response(cx)?;
        // A `u64`'s decimal form is always a valid header value, so the header
        // is only skipped if that ever stops being true.
        if let Ok(value) = HeaderValue::from_str(&self.retry_after_secs.to_string()) {
            response.headers_mut().insert(RETRY_AFTER, value);
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;

    use super::*;

    #[test]
    fn responds_429_with_a_retry_after_header() {
        let response = too_many_requests(60)
            .into_response(&Cx::default())
            .expect("the response builds");

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            response
                .headers()
                .get(RETRY_AFTER)
                .map(HeaderValue::as_bytes),
            Some(&b"60"[..])
        );
    }

    #[test]
    fn keeps_the_retry_after_it_was_built_with() {
        assert_eq!(too_many_requests(30).retry_after_secs(), 30);
    }

    #[test]
    fn names_the_caller_not_the_server() {
        assert_eq!(
            too_many_requests(5).to_string(),
            "too many requests (retry after 5s)"
        );
    }
}
