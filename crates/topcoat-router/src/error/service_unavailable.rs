use http::{HeaderValue, StatusCode, header::RETRY_AFTER};
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Creates a `503 Service Unavailable` error with a `Retry-After` header of
/// `retry_after_secs` seconds.
///
/// Return it when the server is busy for a short time, for example because it
/// sheds load or a dependency is overloaded. Clients and load balancers read a
/// 503 as "busy, try again later", while a
/// [`500 Internal Server Error`](crate::error::internal_server_error) reads as
/// "broken".
///
/// The `Retry-After` header tells clients when to try again, so they do not
/// all invent their own backoff and retry at the same moment.
///
/// # Examples
///
/// ```rust
/// use topcoat::{Result, router::error::service_unavailable};
/// # struct Permit;
/// # fn try_admit() -> Option<Permit> { Some(Permit) }
///
/// async fn handle() -> Result<&'static str> {
///     let Some(_permit) = try_admit() else {
///         return Err(service_unavailable(2).into());
///     };
///
///     Ok("served")
/// }
/// ```
#[must_use]
pub fn service_unavailable(retry_after_secs: u64) -> ServiceUnavailableError {
    ServiceUnavailableError::new(retry_after_secs)
}

/// A `503 Service Unavailable` error.
///
/// Create one with [`service_unavailable`]. Returned from a handler, it renders
/// as a `503 Service Unavailable` response with a `Retry-After` header.
#[derive(Debug, Clone)]
pub struct ServiceUnavailableError {
    retry_after_secs: u64,
}

impl ServiceUnavailableError {
    fn new(retry_after_secs: u64) -> Self {
        Self { retry_after_secs }
    }

    /// Returns the `Retry-After` value of the response, in seconds.
    #[must_use]
    pub fn retry_after_secs(&self) -> u64 {
        self.retry_after_secs
    }
}

impl std::fmt::Display for ServiceUnavailableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "service unavailable (retry after {}s)",
            self.retry_after_secs
        )
    }
}

impl std::error::Error for ServiceUnavailableError {}

impl IntoResponse for ServiceUnavailableError {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let mut response =
            (StatusCode::SERVICE_UNAVAILABLE, "service unavailable").into_response(cx)?;
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
    fn responds_503_with_a_retry_after_header() {
        let response = service_unavailable(2)
            .into_response(&Cx::default())
            .expect("the response builds");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response
                .headers()
                .get(RETRY_AFTER)
                .map(HeaderValue::as_bytes),
            Some(&b"2"[..])
        );
    }

    #[test]
    fn keeps_the_retry_after_it_was_built_with() {
        assert_eq!(service_unavailable(30).retry_after_secs(), 30);
    }

    #[test]
    fn reads_as_busy_rather_than_broken() {
        assert_eq!(
            service_unavailable(5).to_string(),
            "service unavailable (retry after 5s)"
        );
    }
}
