use std::borrow::Cow;
use std::pin::Pin;

use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, Path, Response, Route};

/// The future returned by [`Layer::handle`] and [`Next::run`]: a boxed, `Send`
/// future borrowing the chain and the request context.
pub type LayerFuture<'a> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>;

/// A request-processing layer that wraps the routes nested under its path,
/// similar to a tower middleware.
///
/// A layer wraps every matched route whose path begins with the layer's path —
/// the same prefix rule as layouts — so a layer at `/admin` wraps only routes
/// under `/admin`, while a layer at `/` wraps everything. Each layer receives a
/// mutable [`Cx`] and the request [`Body`], plus a [`Next`] representing the
/// rest of the chain. A layer typically inspects or modifies the context, calls
/// [`Next::run`] to invoke the inner layers and ultimately the route, then
/// inspects or modifies the [`Response`].
///
/// When several layers match a route they nest from least-specific (outermost)
/// to most-specific (innermost), like layouts.
///
/// Register layers with [`RouterBuilder::layer`](crate::RouterBuilder::layer).
///
/// # Examples
///
/// ```rust,ignore
/// use std::borrow::Cow;
/// use topcoat::context::Cx;
/// use topcoat::router::{Body, Layer, LayerFuture, Next, Path};
///
/// struct Timing;
///
/// impl Layer for Timing {
///     fn path(&self) -> Cow<'static, Path> {
///         Cow::Borrowed(Path::new("/"))
///     }
///
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
    /// The URL path prefix whose routes this layer wraps.
    fn path(&self) -> Cow<'static, Path>;

    /// Handles a request, calling `next` to continue down the chain.
    fn handle<'a>(&'a self, cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a>;
}

/// The handler function backing a [`LayerFn`].
pub type LayerHandlerFn = for<'a> fn(cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a>;

/// A [`Layer`] backed by a plain handler function.
///
/// Created either manually via `#[layer("/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a
/// [`RouterBuilder`](crate::RouterBuilder) with
/// [`layer`](crate::RouterBuilder::layer).
#[derive(Debug, Clone)]
pub struct LayerFn {
    /// The URL path prefix whose routes this layer wraps.
    path: Cow<'static, Path>,
    /// The handler function that wraps the inner chain.
    handle: LayerHandlerFn,
}

impl LayerFn {
    /// Creates a new layer with an explicit path prefix and handler function.
    pub const fn new(path: Cow<'static, Path>, handle: LayerHandlerFn) -> Self {
        Self { path, handle }
    }
}

impl Layer for LayerFn {
    fn path(&self) -> Cow<'static, Path> {
        self.path.clone()
    }

    fn handle<'a>(&'a self, cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        (self.handle)(cx, body, next)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(LayerFn);

/// The continuation of a [`Layer`] chain: the remaining layers followed by the
/// matched route handler.
///
/// Passed as the `next` argument to [`Layer::handle`]. Call [`run`](Self::run)
/// to invoke the next layer, or the route once the layers are exhausted.
pub struct Next<'a> {
    layers: &'a [&'a dyn Layer],
    route: &'a dyn Route,
}

impl<'a> Next<'a> {
    /// Creates a chain over `layers` terminating in `route`.
    ///
    /// `layers` must be ordered from least- to most-specific (ascending path
    /// length), so the outermost layer runs first.
    pub(crate) fn new(layers: &'a [&'a dyn Layer], route: &'a dyn Route) -> Self {
        Self { layers, route }
    }

    /// Runs the next layer in the chain, or the route handler once no layers
    /// remain.
    pub fn run(self, cx: &'a mut Cx, body: Body) -> LayerFuture<'a> {
        match self.layers.split_first() {
            Some((layer, layers)) => layer.handle(cx, body, Next { layers, route: self.route }),
            None => self.route.handle(cx, body),
        }
    }
}
