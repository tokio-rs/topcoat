use std::{convert::Infallible, future::ready};

use http::{HeaderMap, header::ACCEPT_ENCODING};
use topcoat_core::context::{Cx, try_context};
use tower::{ServiceExt, service_fn};
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};

use crate::{Body, response::Response};

/// The compression the router applies to a response.
///
/// The router compresses each response body with the algorithm negotiated
/// from the request's `Accept-Encoding` header, after every layer has run.
/// By default, gzip and brotli are enabled at the
/// [`Balanced`](CompressionLevel::Balanced) level, skipping bodies smaller
/// than 32 bytes.
///
/// Responses remain unchanged when the client accepts no enabled algorithm,
/// the body is below [`min_size`](Self::min_size), or its headers indicate
/// that it should not be compressed. This includes already encoded responses,
/// range responses, and content types unsuitable for compression.
///
/// The router reads the configuration from the request context. Pass a
/// configuration to [`RouterBuilder::with`](crate::RouterBuilder::with) to
/// replace the default for the whole application, or to `with(...)` on a
/// route, page, layout, or layer to replace it for that part of the
/// application.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{Compression, CompressionLevel, Router, route},
/// };
///
/// // Serves an archive that is compressed already.
/// #[route(GET "/export.zip", with(Compression::disable()))]
/// async fn export(cx: &Cx) -> Result<Vec<u8>> {
///     Ok(Vec::new())
/// }
///
/// // Trade compression ratio for speed, with gzip only.
/// let router = Router::builder()
///     .with(
///         Compression::default()
///             .brotli(false)
///             .level(CompressionLevel::Fastest),
///     )
///     .route(export)
///     .build();
/// ```
#[derive(Clone, Debug)]
pub struct Compression {
    /// Whether gzip is offered during negotiation.
    gzip: bool,
    /// Whether brotli is offered during negotiation.
    brotli: bool,
    /// The compression quality every algorithm encodes with.
    level: CompressionLevel,
    /// The body size below which responses are not compressed.
    min_size: u64,
}

impl Compression {
    /// Creates a configuration with every algorithm disabled, so responses
    /// are never compressed.
    ///
    /// Use this when something in front of the application compresses
    /// already, like a reverse proxy or CDN.
    #[must_use]
    pub fn disable() -> Self {
        Self {
            gzip: false,
            brotli: false,
            ..Self::default()
        }
    }

    /// Sets whether gzip is offered during negotiation.
    #[must_use]
    pub fn gzip(mut self, enabled: bool) -> Self {
        self.gzip = enabled;
        self
    }

    /// Sets whether brotli is offered during negotiation.
    #[must_use]
    pub fn brotli(mut self, enabled: bool) -> Self {
        self.brotli = enabled;
        self
    }

    /// Sets the compression quality every algorithm encodes with.
    #[must_use]
    pub fn level(mut self, level: CompressionLevel) -> Self {
        self.level = level;
        self
    }

    /// Sets the body size in bytes below which responses are not compressed.
    ///
    /// The limit applies to bodies whose size is known up front (via
    /// `Content-Length` or an exact size hint); a streaming body of unknown
    /// size is always compressed.
    #[must_use]
    pub fn min_size(mut self, bytes: u64) -> Self {
        self.min_size = bytes;
        self
    }

    /// Compresses `response` with the algorithm negotiated from the request's
    /// `Accept-Encoding` values, leaving it unchanged when no enabled
    /// algorithm is accepted or the response should not be compressed.
    pub(crate) async fn compress(
        &self,
        request_headers: &HeaderMap,
        response: Response,
    ) -> Response {
        if !self.gzip && !self.brotli {
            return response;
        }

        // Only the `Accept-Encoding` values matter to the negotiation, so the
        // request handed to the middleware carries just those.
        let mut request = http::Request::new(());
        for value in request_headers.get_all(ACCEPT_ENCODING) {
            request.headers_mut().append(ACCEPT_ENCODING, value.clone());
        }

        // The same skip conditions as tower-http's default, with the size
        // threshold configurable.
        let predicate = SizeAbove::new(self.min_size)
            .and(NotForContentType::GRPC)
            .and(NotForContentType::IMAGES)
            .and(NotForContentType::SSE);

        // The response is already computed; the middleware wraps a one-shot
        // inner service that just yields it.
        let mut response = Some(response);
        let inner = service_fn(move |_: http::Request<()>| {
            let response = response.take().expect("one-shot service called once");
            ready(Ok::<_, Infallible>(response))
        });
        let service = tower_http::compression::Compression::new(inner)
            .gzip(self.gzip)
            .br(self.brotli)
            .quality(self.level.into_tower())
            .compress_when(predicate);

        match service.oneshot(request).await {
            Ok(response) => response.map(Body::new),
            Err(never) => match never {},
        }
    }
}

/// Enables gzip and brotli at the [`Balanced`](CompressionLevel::Balanced)
/// level, skipping bodies smaller than 32 bytes.
impl Default for Compression {
    fn default() -> Self {
        DEFAULT_COMPRESSION.clone()
    }
}

/// The configuration that applies when the context holds none.
static DEFAULT_COMPRESSION: Compression = Compression {
    gzip: true,
    brotli: true,
    level: CompressionLevel::Balanced,
    // Below this size the compressed framing tends to outweigh the savings.
    min_size: 32,
};

/// Returns the [`Compression`] that applies to the response.
///
/// This is the configuration from the most specific `with(...)` that covers
/// the matched route, the configuration passed to
/// [`RouterBuilder::with`](crate::RouterBuilder::with), or the default
/// configuration if there is neither.
#[must_use]
pub fn compression(cx: &Cx) -> &Compression {
    try_context(cx).unwrap_or(&DEFAULT_COMPRESSION)
}

/// The quality a [`Compression`] configuration encodes with, trading
/// compression ratio against CPU time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompressionLevel {
    /// The fastest quality, usually producing the biggest output.
    Fastest,
    /// A quality suited to compressing responses as they are served.
    ///
    /// Balances output size against the CPU time needed for each response.
    #[default]
    Balanced,
    /// The best quality, usually producing the smallest output. With brotli
    /// this is expensive; prefer it for payloads compressed once and cached.
    Best,
    /// A numeric quality interpreted by each algorithm, clamped to the
    /// algorithm's maximum.
    Precise(i32),
}

impl CompressionLevel {
    /// Maps the level onto the middleware's equivalent.
    fn into_tower(self) -> tower_http::CompressionLevel {
        /// The encoding quality used for [`CompressionLevel::Balanced`].
        const BALANCED_QUALITY: i32 = 4;

        match self {
            Self::Fastest => tower_http::CompressionLevel::Fastest,
            Self::Balanced => tower_http::CompressionLevel::Precise(BALANCED_QUALITY),
            Self::Best => tower_http::CompressionLevel::Best,
            Self::Precise(quality) => tower_http::CompressionLevel::Precise(quality),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{borrow::Cow, future::Future};

    use http::{
        HeaderValue,
        header::{CONTENT_ENCODING, VARY},
    };
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{
        HeaderMap, LayerFn, LayerFuture, Method, Next, Path, RouteFn, RouteFuture, RouteHandlerFn,
        Router, request::Bytes, response::IntoResponse, to_bytes,
    };

    // -- Test helpers --

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    /// Builds a router with `handler` under `GET /x` and the given
    /// compression configuration.
    fn router_with(handler: RouteHandlerFn, compression: Compression) -> Router {
        Router::builder()
            .with(compression)
            .route(get("/x", handler))
            .build()
    }

    fn get(path: &'static str, handler: RouteHandlerFn) -> RouteFn {
        RouteFn::new(Method::GET, Cow::Borrowed(Path::new(path)), handler)
    }

    /// Dispatches a GET request for `/x`, optionally with an
    /// `Accept-Encoding` header, and reads the full response.
    fn send(router: &Router, accept_encoding: Option<&str>) -> (HeaderMap, Bytes) {
        send_to(router, "/x", accept_encoding)
    }

    /// Dispatches a GET request for `path`, optionally with an
    /// `Accept-Encoding` header, and reads the full response.
    fn send_to(router: &Router, path: &str, accept_encoding: Option<&str>) -> (HeaderMap, Bytes) {
        let mut request = http::Request::builder().uri(path);
        if let Some(value) = accept_encoding {
            request = request.header(ACCEPT_ENCODING, value);
        }
        let response = block_on(router.handle(request.body(Body::empty()).unwrap()));
        let (parts, body) = response.into_parts();
        let bytes = block_on(to_bytes(body, usize::MAX)).unwrap();
        (parts.headers, bytes)
    }

    /// A body long enough to clear the default compression size threshold.
    fn long_body() -> String {
        "route ".repeat(64)
    }

    fn long_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { long_body().into_response(cx) })
    }

    /// A route whose body stays below the default size threshold.
    fn short_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "route".into_response(cx) })
    }

    /// A route that marks its (uncompressed) response as already encoded.
    fn encoded_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            let mut response = long_body().into_response(cx)?;
            response
                .headers_mut()
                .insert(CONTENT_ENCODING, HeaderValue::from_static("br"));
            Ok(response)
        })
    }

    /// Passes the request on unchanged.
    fn passthrough_layer<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        next.run(cx, body)
    }

    // -- Compression through the router --

    #[test]
    fn compresses_without_a_registered_configuration() {
        let router = Router::builder().route(get("/x", long_route)).build();
        let (headers, _) = send(&router, Some("gzip"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "gzip");
    }

    #[test]
    fn compresses_with_the_negotiated_algorithm() {
        let router = router_with(long_route, Compression::default());

        let (headers, body) = send(&router, Some("gzip"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "gzip");
        assert_eq!(headers.get(VARY).unwrap(), "accept-encoding");
        assert!(!body.is_empty());
        assert!(body.len() < long_body().len());

        let (headers, _) = send(&router, Some("br"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "br");
    }

    #[test]
    fn passes_through_without_accept_encoding() {
        let router = router_with(long_route, Compression::default());
        let (headers, body) = send(&router, None);
        assert!(!headers.contains_key(CONTENT_ENCODING));
        assert_eq!(body, long_body());
    }

    #[test]
    fn off_disables_compression() {
        let router = router_with(long_route, Compression::disable());
        let (headers, body) = send(&router, Some("gzip, br"));
        assert!(!headers.contains_key(CONTENT_ENCODING));
        assert_eq!(body, long_body());
    }

    #[test]
    fn disabled_algorithms_are_not_offered() {
        let router = router_with(long_route, Compression::default().gzip(false));

        // The client only accepts the disabled algorithm.
        let (headers, body) = send(&router, Some("gzip"));
        assert!(!headers.contains_key(CONTENT_ENCODING));
        assert_eq!(body, long_body());

        // The other algorithm is still negotiated.
        let (headers, _) = send(&router, Some("gzip, br"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "br");
    }

    #[test]
    fn small_bodies_are_not_compressed() {
        let router = router_with(short_route, Compression::default());
        let (headers, body) = send(&router, Some("gzip"));
        assert!(!headers.contains_key(CONTENT_ENCODING));
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn min_size_lowers_the_compression_threshold() {
        let router = router_with(short_route, Compression::default().min_size(0));
        let (headers, _) = send(&router, Some("gzip"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "gzip");
    }

    #[test]
    fn already_encoded_responses_pass_through() {
        let router = router_with(encoded_route, Compression::default());
        let (headers, body) = send(&router, Some("gzip"));
        // The route's own encoding wins; the body is not compressed again.
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "br");
        assert_eq!(body, long_body());
    }

    #[test]
    fn a_route_configuration_replaces_the_router_configuration() {
        let router = Router::builder()
            .with(Compression::disable())
            .route(get("/x", long_route).with(Compression::default()))
            .route(get("/y", long_route))
            .build();

        let (headers, _) = send_to(&router, "/x", Some("gzip"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "gzip");
        let (headers, _) = send_to(&router, "/y", Some("gzip"));
        assert!(!headers.contains_key(CONTENT_ENCODING));
    }

    #[test]
    fn a_layer_configuration_applies_to_the_routes_under_it() {
        let router = Router::builder()
            .layer(
                LayerFn::new(Some(Path::new("/quiet")), passthrough_layer)
                    .with(Compression::disable()),
            )
            .route(get("/quiet/x", long_route))
            .route(get("/x", long_route))
            .build();

        let (headers, _) = send_to(&router, "/quiet/x", Some("gzip"));
        assert!(!headers.contains_key(CONTENT_ENCODING));
        let (headers, _) = send_to(&router, "/x", Some("gzip"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "gzip");
    }
}
