use std::borrow::Cow;

use topcoat_core::context::{Cx, CxBuilder, try_request_context};

use crate::{Body, IntoPath, Layer, LayerFuture, Next, Path};

/// The body limit in bytes applied when no [`BodyLimit`] layer matches the
/// request.
pub(crate) const DEFAULT_BODY_LIMIT: usize = 2 * 1024 * 1024;

/// A router layer that overrides the request body size limit for the routes
/// under its path.
///
/// Extractors that buffer the request body ([`Bytes`](crate::Bytes),
/// [`Json`](crate::content::Json), [`Form`](crate::content::Form), and the
/// other built-ins) read at most the request's body limit and reject a larger
/// body with `413 Content Too Large`, so a client cannot exhaust the server's
/// memory. The limit defaults to 2 MiB and applies without any configuration;
/// register this layer to change it.
///
/// Create the layer with [`max`](Self::max) or [`disable`](Self::disable) and
/// register it with [`RouterBuilder::layer`](crate::RouterBuilder::layer). It
/// covers the whole application by default; scope it to a path prefix with
/// [`at`](Self::at). Layers nest from least- to most-specific path, so a
/// scoped layer overrides a broader one for the routes it wraps.
///
/// The limit only applies where a buffering extractor enforces it: a handler
/// that takes the raw [`Body`] streams the request instead and reads it on
/// its own terms.
///
/// # Examples
///
/// ```rust
/// use topcoat::router::{BodyLimit, Router};
///
/// let router = Router::builder()
///     // Allow up to 32 MiB under /upload, keep the 2 MiB default elsewhere.
///     .layer(BodyLimit::max(32 * 1024 * 1024).at("/upload"))
///     .build();
/// ```
#[derive(Debug, Clone)]
#[must_use]
pub struct BodyLimit {
    /// The URL path prefix whose routes this layer applies to.
    path: Cow<'static, Path>,
    /// The limit the layer registers on the request context.
    kind: BodyLimitKind,
}

impl BodyLimit {
    /// Creates a layer that limits request bodies to `limit` bytes.
    pub const fn max(limit: usize) -> Self {
        Self {
            path: Cow::Borrowed(Path::new("/")),
            kind: BodyLimitKind::Limit(limit),
        }
    }

    /// Creates a layer that turns the body limit off, letting extractors
    /// buffer a body of any size.
    pub const fn disable() -> Self {
        Self {
            path: Cow::Borrowed(Path::new("/")),
            kind: BodyLimitKind::Disable,
        }
    }

    /// Scopes the layer to the routes under `path`.
    ///
    /// # Panics
    ///
    /// Panics if `path` is a string that is not a well-formed route path.
    pub fn at(mut self, path: impl IntoPath) -> Self {
        self.path = path.into_path();
        self
    }
}

impl Layer for BodyLimit {
    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        cx.insert(self.kind);
        next.run(cx, body)
    }
}

/// The limit a [`BodyLimit`] layer registers on the request context.
#[derive(Debug, Clone, Copy)]
enum BodyLimitKind {
    /// No limit; extractors buffer a body of any size.
    Disable,
    /// The maximum number of bytes an extractor buffers.
    Limit(usize),
}

/// Returns the request's effective body size limit in bytes.
///
/// This is the limit registered by the innermost [`BodyLimit`] layer wrapping
/// the matched route, the 2 MiB default when no layer matches, or
/// [`usize::MAX`] when the limit is disabled.
///
/// The built-in buffering extractors enforce this limit already. In a custom
/// [`FromRequest`](crate::FromRequest) implementation, prefer delegating the
/// buffering to [`Bytes`](crate::Bytes), which enforces it too; pass this
/// value to [`to_bytes`](crate::to_bytes) when reading the body by hand.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{Body, FromRequest, body_limit, error::bad_request, to_bytes},
/// };
///
/// struct Raw(Vec<u8>);
///
/// impl FromRequest for Raw {
///     async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
///         let bytes = to_bytes(body, body_limit(cx))
///             .await
///             .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;
///
///         Ok(Self(bytes.into()))
///     }
/// }
/// ```
#[must_use]
pub fn body_limit(cx: &Cx) -> usize {
    match try_request_context(cx) {
        Some(BodyLimitKind::Limit(limit)) => *limit,
        Some(BodyLimitKind::Disable) => usize::MAX,
        None => DEFAULT_BODY_LIMIT,
    }
}

#[cfg(test)]
mod tests {
    use http::{Method, Request, StatusCode};
    use topcoat_core::context::CxTestBuilder;

    use super::*;
    use crate::{
        Bytes, FromRequest, IntoResponse, Response, RouteFn, RouteFuture, Router, to_bytes,
    };

    // -- body_limit --

    #[test]
    fn body_limit_defaults_to_two_mebibytes() {
        assert_eq!(body_limit(&Cx::default()), DEFAULT_BODY_LIMIT);
    }

    #[test]
    fn body_limit_reads_the_registered_limit() {
        let cx = CxTestBuilder::new()
            .request_context(BodyLimitKind::Limit(16))
            .build();
        assert_eq!(body_limit(&cx), 16);
    }

    #[test]
    fn body_limit_disabled_is_unlimited() {
        let cx = CxTestBuilder::new()
            .request_context(BodyLimitKind::Disable)
            .build();
        assert_eq!(body_limit(&cx), usize::MAX);
    }

    // -- BodyLimit as a layer --

    #[test]
    fn layer_defaults_to_the_root_path() {
        assert_eq!(BodyLimit::max(1).path(), Path::new("/"));
        assert_eq!(BodyLimit::disable().path(), Path::new("/"));
    }

    #[test]
    fn at_scopes_the_layer_path() {
        assert_eq!(BodyLimit::max(1).at("/upload").path(), Path::new("/upload"));
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
    async fn max_overrides_the_default_limit() {
        let router = Router::builder()
            .route(echo_route("/echo"))
            .layer(BodyLimit::max(8))
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
            .route(echo_route("/echo"))
            .layer(BodyLimit::disable())
            .build();

        let response = send(&router, "/echo", DEFAULT_BODY_LIMIT + 1).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn a_scoped_layer_overrides_an_outer_one() {
        let router = Router::builder()
            .route(echo_route("/echo"))
            .route(echo_route("/upload"))
            .layer(BodyLimit::max(8))
            .layer(BodyLimit::max(64).at("/upload"))
            .build();

        // The scoped layer runs innermost, so its limit wins under /upload...
        assert_eq!(send(&router, "/upload", 64).await.status(), StatusCode::OK);
        // ...while other routes keep the outer limit.
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
            .route(RouteFn::new(Method::POST, "/raw", raw))
            .layer(BodyLimit::max(4))
            .build();

        assert_eq!(send(&router, "/raw", 1024).await.status(), StatusCode::OK);
    }
}
