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
    /// Creates the default configuration: gzip and brotli enabled at each
    /// algorithm's default level, skipping bodies smaller than 32 bytes.
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
    /// Each algorithm's own default quality.
    #[default]
    Default,
    /// The best quality, usually producing the smallest output.
    Best,
    /// A numeric quality interpreted by each algorithm, clamped to the
    /// algorithm's maximum.
    Precise(i32),
}

impl CompressionLevel {
    /// Maps the level onto the middleware's equivalent.
    fn into_tower(self) -> tower_http::CompressionLevel {
        match self {
            Self::Fastest => tower_http::CompressionLevel::Fastest,
            Self::Default => tower_http::CompressionLevel::Default,
            Self::Best => tower_http::CompressionLevel::Best,
            Self::Precise(quality) => tower_http::CompressionLevel::Precise(quality),
        }
    }
}
