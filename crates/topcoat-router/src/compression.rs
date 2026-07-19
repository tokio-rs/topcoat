use std::convert::Infallible;
use std::future::ready;

use http::HeaderMap;
use http::header::ACCEPT_ENCODING;
use tower::{ServiceExt, service_fn};
use tower_http::compression::predicate::{NotForContentType, Predicate, SizeAbove};

use crate::{Body, Response};

/// Configures the compression a [`Router`](crate::Router) applies to
/// responses.
///
/// The router compresses each response body with the algorithm negotiated
/// from the request's `Accept-Encoding` header, after every layer has run.
/// Compression is enabled by default; pass a configuration to
/// [`RouterBuilder::compression`](crate::RouterBuilder::compression) to tune
/// or disable it.
///
/// A response is passed through unchanged when the client accepts no enabled
/// algorithm, or when compressing it would be wasteful or incorrect: it is
/// already encoded (`Content-Encoding`), it is a range (`Content-Range`), its
/// content type is an image (except SVG), a gRPC message, or an event stream,
/// or its body is known to be smaller than [`min_size`](Self::min_size).
///
/// # Examples
///
/// ```rust
/// use topcoat::router::{Compression, CompressionLevel, Router};
///
/// // Disable compression, e.g. when a reverse proxy compresses instead.
/// let router = Router::builder().compression(Compression::off()).build();
///
/// // Trade compression ratio for speed, with gzip only.
/// let router = Router::builder()
///     .compression(
///         Compression::new()
///             .brotli(false)
///             .level(CompressionLevel::Fastest),
///     )
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
    /// Creates the default configuration: gzip and brotli enabled at the
    /// [`Balanced`](CompressionLevel::Balanced) level, skipping bodies
    /// smaller than 32 bytes.
    #[must_use]
    pub fn new() -> Self {
        /// Below this size the compressed framing tends to outweigh the
        /// savings.
        const DEFAULT_MIN_SIZE: u64 = 32;

        Self {
            gzip: true,
            brotli: true,
            level: CompressionLevel::default(),
            min_size: DEFAULT_MIN_SIZE,
        }
    }

    /// Creates a configuration with every algorithm disabled, so responses
    /// are never compressed.
    ///
    /// Use this when something in front of the application compresses
    /// already, like a reverse proxy or CDN.
    #[must_use]
    pub fn off() -> Self {
        Self {
            gzip: false,
            brotli: false,
            ..Self::new()
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

impl Default for Compression {
    fn default() -> Self {
        Self::new()
    }
}

/// The quality a [`Compression`] configuration encodes with, trading
/// compression ratio against CPU time.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CompressionLevel {
    /// The fastest quality, usually producing the biggest output.
    Fastest,
    /// A speed-leaning quality suited to compressing responses on the fly.
    ///
    /// This is deliberately not each algorithm's own default: brotli's is its
    /// highest quality, tuned for compressing assets ahead of time, and far
    /// too slow to run per response.
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
        /// The quality [`CompressionLevel::Balanced`] encodes with: level 4
        /// for both gzip (of 0-9) and brotli (of 0-11) compresses at
        /// rendering speed while brotli still beats gzip's best ratio.
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
    use std::borrow::Cow;
    use std::future::Future;

    use http::HeaderValue;
    use http::header::{CONTENT_ENCODING, VARY};
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{
        Bytes, HeaderMap, IntoResponse, Method, Path, RouteFn, RouteFuture, RouteHandlerFn, Router,
        to_bytes,
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
            .route(RouteFn::new(
                Method::GET,
                Cow::Borrowed(Path::new("/x")),
                handler,
            ))
            .compression(compression)
            .build()
    }

    /// Dispatches a GET request for `/x`, optionally with an
    /// `Accept-Encoding` header, and reads the full response.
    fn send(router: &Router, accept_encoding: Option<&str>) -> (HeaderMap, Bytes) {
        let mut request = http::Request::builder().uri("/x");
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

    // -- Compression through the router --

    #[test]
    fn compresses_with_the_negotiated_algorithm() {
        let router = router_with(long_route, Compression::new());

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
        let router = router_with(long_route, Compression::new());
        let (headers, body) = send(&router, None);
        assert!(!headers.contains_key(CONTENT_ENCODING));
        assert_eq!(body, long_body());
    }

    #[test]
    fn off_disables_compression() {
        let router = router_with(long_route, Compression::off());
        let (headers, body) = send(&router, Some("gzip, br"));
        assert!(!headers.contains_key(CONTENT_ENCODING));
        assert_eq!(body, long_body());
    }

    #[test]
    fn disabled_algorithms_are_not_offered() {
        let router = router_with(long_route, Compression::new().gzip(false));

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
        let router = router_with(short_route, Compression::new());
        let (headers, body) = send(&router, Some("gzip"));
        assert!(!headers.contains_key(CONTENT_ENCODING));
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn min_size_lowers_the_compression_threshold() {
        let router = router_with(short_route, Compression::new().min_size(0));
        let (headers, _) = send(&router, Some("gzip"));
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "gzip");
    }

    #[test]
    fn already_encoded_responses_pass_through() {
        let router = router_with(encoded_route, Compression::new());
        let (headers, body) = send(&router, Some("gzip"));
        // The route's own encoding wins; the body is not compressed again.
        assert_eq!(headers.get(CONTENT_ENCODING).unwrap(), "br");
        assert_eq!(body, long_body());
    }
}
