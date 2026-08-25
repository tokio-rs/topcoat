use http::{HeaderValue, StatusCode, header::RETRY_AFTER};
use topcoat_core::{context::Cx, error::Result};

use crate::response::{IntoResponse, Response};

/// Builds a too-many-requests (HTTP 429) response carrying a `Retry-After`
/// hint, in seconds.
///
/// Return this when a caller has exceeded a limit you set for them: a rate
/// limit, a quota, a per-account cap. It says the request was refused because
/// of who sent it and how often, which is what separates it from
/// [`service_unavailable`](crate::error::service_unavailable) — that one says
/// the server as a whole is at capacity, and applies to every caller at once.
/// A client that can tell the two apart can back off its own traffic in the
/// first case and fail over in the second.
///
/// `Retry-After` is what makes the refusal actionable. Without it a rate
/// limiter teaches callers nothing except to retry immediately.
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

/// A too-many-requests response carried as the `Err` variant of a handler
/// `Result`.
///
/// Construct one with [`too_many_requests`].
#[derive(Debug)]
pub struct TooManyRequestsError {
    retry_after_secs: u64,
}

impl TooManyRequestsError {
    fn new(retry_after_secs: u64) -> Self {
        Self { retry_after_secs }
    }

    /// The `Retry-After` value this response carries, in seconds.
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
    fn into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send {
        // A `u64`'s decimal form is always a valid header value, so the header
        // is only skipped if that ever stops being true.
        let retry_after = HeaderValue::from_str(&self.retry_after_secs.to_string())
            .ok()
            .map(|value| [(RETRY_AFTER, value)]);
        (StatusCode::TOO_MANY_REQUESTS, retry_after, "too many requests").into_response(cx)
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
