mod rerun;
#[cfg(not(target_family = "wasm"))]
mod socket;

pub use rerun::*;
use topcoat_core::context::Cx;
use topcoat_router::{Body, Layer, LayerFuture, Next, Path};

/// The WebSocket subprotocol for runtime connections.
///
/// Request this subprotocol at a page's URL to open a connection through
/// [`RuntimeLayer`]. The browser can then request renders and receive the
/// page's content as frames. WebSocket connections require a native server.
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
///   connection runs the page as a `GET`, using headers from the handshake and the signal values
///   sent by the browser.
///
/// WebSocket connections are supported on native servers. HTTP page reruns
/// are also available on WebAssembly.
///
/// Other requests pass through unchanged, including form submissions and
/// WebSocket requests that do not use the runtime subprotocol.
///
/// Register this layer with
/// [`RouterBuilderRuntimeExt::runtime`](crate::RouterBuilderRuntimeExt::runtime).
/// Call it after adding your application's pathless layers so they receive
/// the rewritten `GET`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeLayer;

impl Layer for RuntimeLayer {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        #[cfg(not(target_family = "wasm"))]
        if socket::requested(cx) {
            return Box::pin(socket::accept(cx, body));
        }
        if rerun::requested(cx) {
            return Box::pin(rerun::dispatch(cx, body));
        }
        next.run(cx, body)
    }
}
