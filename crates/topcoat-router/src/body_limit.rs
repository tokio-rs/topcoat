use topcoat_core::context::{Cx, try_context};

/// The body limit in bytes applied when the context holds no [`BodyLimit`].
pub(crate) const DEFAULT_BODY_LIMIT: usize = 2 * 1024 * 1024;

/// The request body size limit.
///
/// Buffering extractors reject bodies above the limit with
/// `413 Content Too Large`. The default limit is 2 MiB.
///
/// The router reads the limit from the request context. Pass a limit to
/// [`RouterBuilder::with`](crate::RouterBuilder::with) to replace the default
/// for the whole application, or to a route's `with(...)` to replace it for
/// that route. A layout's `with(...)` applies to the pages under it.
///
/// The limit only applies where a buffering extractor enforces it: a handler
/// that takes the raw [`Body`](crate::Body) streams the request instead and
/// reads it on its own terms.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{BodyLimit, Router, request::Bytes, route},
/// };
///
/// // Accepts uploads of up to 32 MiB.
/// #[route(POST "/upload", with(BodyLimit::max(32 * 1024 * 1024)))]
/// async fn upload(cx: &Cx, bytes: Bytes) -> Result<()> {
///     Ok(())
/// }
///
/// let router = Router::builder()
///     .with(BodyLimit::max(64 * 1024))
///     .route(upload)
///     .build();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct BodyLimit {
    /// The maximum number of bytes an extractor buffers.
    max: usize,
}

impl BodyLimit {
    /// Creates a limit of `max` bytes.
    pub const fn max(max: usize) -> Self {
        Self { max }
    }

    /// Creates a limit that is turned off, letting extractors buffer a body
    /// of any size.
    pub const fn disable() -> Self {
        Self { max: usize::MAX }
    }
}

/// Limits bodies to 2 MiB.
impl Default for BodyLimit {
    fn default() -> Self {
        Self::max(DEFAULT_BODY_LIMIT)
    }
}

/// Returns the request's body size limit in bytes.
///
/// This is the limit from the most specific `with(...)` that covers the
/// matched route, the limit passed to
/// [`RouterBuilder::with`](crate::RouterBuilder::with), or the default limit
/// if there is neither. A disabled limit is [`usize::MAX`].
///
/// The built-in buffering extractors enforce this limit already. In a custom
/// [`FromRequest`](crate::request::FromRequest) implementation, prefer delegating the
/// buffering to [`Bytes`](crate::request::Bytes), which enforces it too; pass this
/// value to [`to_bytes`](crate::to_bytes) when reading the body by hand.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{Body, body_limit, request::FromRequest, to_bytes},
/// };
///
/// struct Raw(Vec<u8>);
///
/// impl FromRequest for Raw {
///     async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
///         let bytes = to_bytes(body, body_limit(cx)).await?;
///
///         Ok(Self(bytes.into()))
///     }
/// }
/// ```
#[must_use]
pub fn body_limit(cx: &Cx) -> usize {
    try_context::<BodyLimit>(cx).map_or(DEFAULT_BODY_LIMIT, |limit| limit.max)
}

#[cfg(test)]
mod tests {
    use http::{Method, Request, StatusCode};
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{
        Body, RouteFn, RouteFuture, Router,
        request::{Bytes, FromRequest},
        response::{IntoResponse, Response},
        to_bytes,
    };

    // -- body_limit --

    #[test]
    fn body_limit_reads_the_context_limit() {
        let cx = Cx::new().with(BodyLimit::max(16));
        assert_eq!(body_limit(&cx), 16);
    }

    #[test]
    fn body_limit_disabled_is_unlimited() {
        let cx = Cx::new().with(BodyLimit::disable());
        assert_eq!(body_limit(&cx), usize::MAX);
    }

    #[test]
    fn body_limit_without_a_limit_matches_the_default() {
        assert_eq!(body_limit(&Cx::new()), BodyLimit::default().max);
    }

    // -- Test helpers --

    /// A route that buffers its body through the `Bytes` extractor, answering
    /// with the byte count.
    fn echo(cx: &Cx, body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            let bytes = Bytes::from_request(cx, body).await?;
            bytes.len().to_string().into_response(cx)
        })
    }

    fn echo_route(path: &'static str) -> RouteFn {
        RouteFn::new(Method::POST, path, echo)
    }

    /// Dispatches a POST request carrying a body of `size` zero bytes.
    async fn send(router: &Router, path: &str, size: usize) -> Response {
        let request = Request::builder()
            .method(Method::POST)
            .uri(path)
            .body(Body::from(vec![0u8; size]))
            .expect("request should build");
        router.handle(request).await
    }

    // -- The limit through the router --

    #[tokio::test]
    async fn requests_within_the_default_limit_pass() {
        let router = Router::builder().route(echo_route("/echo")).build();
        assert_eq!(send(&router, "/echo", 1024).await.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn requests_over_the_default_limit_are_content_too_large() {
        let router = Router::builder().route(echo_route("/echo")).build();
        let response = send(&router, "/echo", DEFAULT_BODY_LIMIT + 1).await;

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert_eq!(&body[..], b"content too large");
    }

    #[tokio::test]
    async fn a_router_limit_applies_to_every_route() {
        let router = Router::builder()
            .with(BodyLimit::max(8))
            .route(echo_route("/echo"))
            .build();

        // A body at the limit passes; one past it is rejected.
        assert_eq!(send(&router, "/echo", 8).await.status(), StatusCode::OK);
        assert_eq!(
            send(&router, "/echo", 9).await.status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn disable_turns_the_limit_off() {
        let router = Router::builder()
            .with(BodyLimit::disable())
            .route(echo_route("/echo"))
            .build();

        let response = send(&router, "/echo", DEFAULT_BODY_LIMIT + 1).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_route_limit_replaces_the_router_limit() {
        let router = Router::builder()
            .with(BodyLimit::max(8))
            .route(echo_route("/echo"))
            .route(echo_route("/upload").with(BodyLimit::max(64)))
            .build();

        assert_eq!(send(&router, "/upload", 64).await.status(), StatusCode::OK);
        assert_eq!(
            send(&router, "/echo", 64).await.status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    #[tokio::test]
    async fn the_raw_body_extractor_is_not_limited() {
        fn raw(cx: &Cx, body: Body) -> RouteFuture<'_> {
            Box::pin(async move {
                let bytes = to_bytes(body, usize::MAX).await.expect("body reads fully");
                bytes.len().to_string().into_response(cx)
            })
        }

        let router = Router::builder()
            .with(BodyLimit::max(4))
            .route(RouteFn::new(Method::POST, "/raw", raw))
            .build();

        assert_eq!(send(&router, "/raw", 1024).await.status(), StatusCode::OK);
    }
}
