use http::{HeaderValue, StatusCode, header::RETRY_AFTER};
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Builds a service-unavailable (HTTP 503) response carrying a `Retry-After`
/// hint, in seconds.
///
/// Return this when the server is temporarily at capacity: load shedding,
/// admission control, or a saturated dependency. It is deliberately distinct
/// from [`internal_server_error`](crate::error::internal_server_error), which
/// tells a client, a load balancer, and an on-call engineer that something is
/// broken. Answering "broken" when the truth is "busy" reads as an outage to
/// everything automated downstream.
///
/// `Retry-After` is why the constructor takes an argument. A bare 503 tells a
/// caller to go away without saying when to come back, so every well-behaved
/// client invents its own backoff and they all synchronize.
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

/// A service-unavailable response carried as the `Err` variant of a handler
/// `Result`.
///
/// Construct one with [`service_unavailable`].
#[derive(Debug)]
pub struct ServiceUnavailableError {
    retry_after_secs: u64,
}

impl ServiceUnavailableError {
    fn new(retry_after_secs: u64) -> Self {
        Self { retry_after_secs }
    }

    /// The `Retry-After` value this response carries, in seconds.
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
    fn into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        // A `u64`'s decimal form is always a valid header value, so the header
        // is only skipped if that ever stops being true.
        let retry_after = HeaderValue::from_str(&self.retry_after_secs.to_string())
            .ok()
            .map(|value| [(RETRY_AFTER, value)]);
        (
            StatusCode::SERVICE_UNAVAILABLE,
            retry_after,
            "service unavailable",
        )
            .into_response(cx)
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;

    use super::*;

    #[tokio::test]
    async fn responds_503_with_a_retry_after_header() {
        let response = service_unavailable(2)
            .into_response(&Cx::default())
            .await
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
