mod rerun;
mod socket;

pub use rerun::*;
pub use socket::*;
use topcoat_core::context::Cx;
use topcoat_router::{Body, Layer, LayerFuture, Next, Path};

/// A [`Layer`] that serves the runtime protocol at page URLs.
///
/// The protocol has two transports, both addressed to the page's own URL:
///
/// - A page rerun: a `POST` with `X-Topcoat-Runtime: true` and a JSON body containing the
///   document's signal values. The layer rewrites it to a `GET` at the same path and query. The
///   page runs through its layouts and guards, and its signals resume from the supplied values. The
///   rewritten request has an empty body, and its [`RUNTIME_HEADER`], `Content-Type`, and
///   `Content-Length` headers are removed. Read the client's original method with
///   [`original_method`](topcoat_router::request::original_method).
/// - A connection: a WebSocket handshake requesting the [`RUNTIME_PROTOCOL`] subprotocol. The layer
///   accepts it and renders the page over the connection each time the browser asks, as a `GET`
///   with the handshake's headers and the supplied signal values.
///
/// Every other request passes through unchanged, including ordinary form
/// submissions and the application's own WebSocket routes.
///
/// [`RouterBuilderRuntimeExt::runtime`](crate::RouterBuilderRuntimeExt::runtime)
/// registers this layer. Call it after registering your application's
/// pathless layers so those layers receive the rewritten `GET`.
#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeLayer;

impl Layer for RuntimeLayer {
    fn path(&self) -> Option<&Path> {
        None
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        if socket::requested(cx) {
            return Box::pin(socket::accept(cx, body));
        }
        if rerun::requested(cx) {
            return Box::pin(rerun::dispatch(cx, body));
        }
        next.run(cx, body)
    }
}
