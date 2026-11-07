mod rerun;
#[cfg(not(target_family = "wasm"))]
mod socket;

pub use rerun::*;
use topcoat_core::context::Cx;
use topcoat_router::{Body, Layer, LayerFuture, Next, Path};

use crate::RuntimeSetup;

/// The WebSocket subprotocol for runtime connections.
///
/// Request this subprotocol at a page's URL to open a connection through
/// [`RuntimeLayer`]. The browser can then request renders and receive the
/// content as frames. WebSocket connections require a native server.
pub const RUNTIME_PROTOCOL: &str = "topcoat-runtime";

/// A [`Layer`] that handles runtime requests at each page's URL.
///
/// The browser can request a render in two ways:
///
/// - Send a `POST` with `X-Topcoat-Runtime: true` and the document's signal values as JSON. The
///   layer rewrites this to a `GET` at the same path and query. The page, layouts, and guards run
///   with the supplied signal values. The rewritten request has an empty body and no
///   [`RUNTIME_HEADER`], `Content-Type`, or `Content-Length` headers. To read the original method,
///   use [`original_method`](topcoat_router::request::original_method).
/// - Open a WebSocket with the [`RUNTIME_PROTOCOL`] subprotocol. Each render request on this
///   connection describes an HTTP request: a method, a path, a body, and a few runtime headers,
///   such as [`RUNTIME_HEADER`] for a page rerun. The layer dispatches it with the handshake's
///   other headers as a connected render. Renders run side by side, and each message sent back
///   names the render it belongs to. A connection may have at most
///   [`max_runs_per_connection`](Self::max_runs_per_connection) renders at once.
///
/// WebSocket connections are supported on native servers. HTTP page reruns
/// are also available on WebAssembly.
///
/// Other requests pass through unchanged, including form submissions and
/// WebSocket requests that do not use the runtime subprotocol. The layer adds
/// [`RuntimeSetup`] to their context.
///
/// Add it after your application's pathless layers so they receive the
/// rewritten `GET`.
///
/// ```rust
/// use topcoat_router::Router;
/// use topcoat_runtime::RuntimeLayer;
///
/// let router = Router::builder().layer(RuntimeLayer::new()).build();
/// ```
#[derive(Debug, Clone, Copy)]
pub struct RuntimeLayer {
    // Read only by the WebSocket handler, which native servers compile.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    max_runs_per_connection: usize,
}

impl RuntimeLayer {
    /// Creates a runtime layer that allows 64 renders at once on each
    /// connection.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_runs_per_connection: 64,
        }
    }

    /// Limits the number of renders running at once on a runtime connection.
    /// The default limit is 64.
    ///
    /// Each connected page or shard has a render, unless it is inside another
    /// connected page or shard. At the limit, new render requests receive
    /// `429 Too Many Requests`. They can be accepted again once a render
    /// finishes or stops.
    ///
    /// ```rust
    /// use topcoat_router::Router;
    /// use topcoat_runtime::RuntimeLayer;
    ///
    /// let router = Router::builder()
    ///     .layer(RuntimeLayer::new().max_runs_per_connection(128))
    ///     .build();
    /// ```
    #[must_use]
    pub const fn max_runs_per_connection(mut self, max: usize) -> Self {
        self.max_runs_per_connection = max;
        self
    }
}

impl Default for RuntimeLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl Layer for RuntimeLayer {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        #[cfg(not(target_family = "wasm"))]
        if socket::requested(cx) {
            return Box::pin(socket::accept(cx, body, self.max_runs_per_connection));
        }
        if rerun::requested(cx) {
            return Box::pin(rerun::dispatch(cx, body));
        }
        let cx = cx.with(RuntimeSetup);
        Box::pin(async move { next.run(&cx, body).await })
    }
}
