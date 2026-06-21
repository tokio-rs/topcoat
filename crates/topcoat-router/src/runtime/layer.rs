use std::pin::Pin;

use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, Response, Route};

/// The future returned by [`Layer::handle`] and [`Next::run`]: a boxed, `Send`
/// future borrowing the chain and the request context.
pub type LayerFuture<'a> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>;

/// A request-processing layer that wraps the matched route, similar to a tower
/// middleware.
///
/// Each layer receives a mutable [`Cx`] and the request [`Body`], plus a
/// [`Next`] representing the rest of the chain. A layer typically inspects or
/// modifies the context, calls [`Next::run`] to invoke the inner layers and
/// ultimately the route, then inspects or modifies the [`Response`].
///
/// Register layers with [`RouterBuilder::layer`](crate::RouterBuilder::layer).
/// They nest like an onion: the most recently registered layer is the
/// outermost and runs first.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::context::Cx;
/// use topcoat::router::{Body, Layer, LayerFuture, Next};
///
/// struct Timing;
///
/// impl Layer for Timing {
///     fn handle<'a>(&'a self, cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
///         Box::pin(async move {
///             let start = std::time::Instant::now();
///             let response = next.run(cx, body).await?;
///             println!("handled in {:?}", start.elapsed());
///             Ok(response)
///         })
///     }
/// }
/// ```
pub trait Layer: Send + Sync + 'static {
    /// Handles a request, calling `next` to continue down the chain.
    fn handle<'a>(&'a self, cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a>;
}

/// The continuation of a [`Layer`] chain: the remaining layers followed by the
/// matched route handler.
///
/// Passed as the `next` argument to [`Layer::handle`]. Call [`run`](Self::run)
/// to invoke the next layer, or the route once the layers are exhausted.
pub struct Next<'a> {
    layers: &'a [Box<dyn Layer>],
    route: &'a dyn Route,
}

impl<'a> Next<'a> {
    /// Creates a chain over `layers` terminating in `route`.
    pub(crate) fn new(layers: &'a [Box<dyn Layer>], route: &'a dyn Route) -> Self {
        Self { layers, route }
    }

    /// Runs the next layer in the chain, or the route handler once no layers
    /// remain.
    ///
    /// Layers are consumed from the end, so the most recently registered layer
    /// is the outermost and runs first.
    pub fn run(self, cx: &'a mut Cx, body: Body) -> LayerFuture<'a> {
        match self.layers.split_last() {
            Some((layer, layers)) => layer.handle(cx, body, Next { layers, route: self.route }),
            None => self.route.handle(cx, body),
        }
    }
}
