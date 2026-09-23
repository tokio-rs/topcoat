use std::{hash::Hash, pin::Pin};

use serde::Deserialize;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, Method, Methods, Path, PathBuf, Route, RouteFuture, RouteId, RouterBuilder,
    response::IntoResponse,
};
use topcoat_view::ViewHandle;

use crate::{Arguments, SignalValues};

pub(crate) const SHARD_ROUTE_PREFIX: &str = "/_topcoat/runtime/shards";

/// The body of a request that re-renders a shard: the current values of its
/// arguments and of the signals its content created.
///
/// The identity of the shard invocation is not part of the body. It travels
/// in the request's identity header.
#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "Arguments<A>: Deserialize<'de>"))]
pub struct ShardRequest<A> {
    args: Arguments<A>,
    #[serde(default)]
    signals: SignalValues,
}

impl<A> ShardRequest<A> {
    /// Splits the request into the arguments for the shard body and the
    /// signal values to register on its request context.
    pub fn into_parts(self) -> (A, SignalValues) {
        (self.args.0, self.signals)
    }
}

/// The identity of a shard, shared by the server and the browser runtime.
///
/// The id names the shard's route. `#[shard]` generates a unique id for each
/// shard.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShardId(&'static str);

impl ShardId {
    /// Creates an id from its string form.
    #[must_use]
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    /// Returns the id as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0
    }
}

/// The future returned by [`Shard::render`]: a boxed, `Send` future borrowing
/// the shard and its request context.
pub type ShardFuture<'cx> = Pin<Box<dyn Future<Output = Result<ViewHandle>> + Send + 'cx>>;

/// A component that re-renders on the server when its inputs change in the
/// browser.
///
/// `#[shard]` implements this trait. Register a shard on a [`RouterBuilder`]
/// with [`shard`](RouterBuilderShardExt::shard), which serves it on a route
/// named after its [`ShardId`].
pub trait Shard: Send + Sync + 'static {
    /// The identity of this shard.
    fn id(&self) -> ShardId;

    /// Renders the shard for a re-render request, reading its arguments and
    /// signal values from the JSON `body`.
    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> ShardFuture<'cx>;
}

impl<S: Shard + ?Sized> Shard for &'static S {
    fn id(&self) -> ShardId {
        (**self).id()
    }

    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> ShardFuture<'cx> {
        (**self).render(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Shard);

/// A [`Route`] that re-renders one shard.
///
/// [`RouterBuilderShardExt::shard`] creates and registers one.
pub struct ShardRoute {
    id: RouteId,
    path: PathBuf,
    shard: Box<dyn Shard>,
}

impl ShardRoute {
    /// Creates the route that serves `shard`.
    pub fn new(shard: impl Shard) -> Self {
        Self {
            id: RouteId::new(),
            path: Path::new(&format!("{SHARD_ROUTE_PREFIX}/{}", shard.id().as_str())).to_owned(),
            shard: Box::new(shard),
        }
    }
}

impl Route for ShardRoute {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        // Avoids URL length limits for large parameters.
        Methods::Only(&[Method::POST])
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            let view = self.shard.render(cx, body).await?;
            view.into_response(cx)
        })
    }
}

/// Registers shards on a [`RouterBuilder`].
pub trait RouterBuilderShardExt {
    /// Mounts the route that serves `shard`.
    #[must_use]
    fn shard(self, shard: impl Shard) -> Self;

    /// Registers every `#[shard]` linked into the binary.
    ///
    /// The `discover` method of the router builder already calls this.
    #[cfg(feature = "discover")]
    #[must_use]
    fn discover_shards(self) -> Self;
}

impl RouterBuilderShardExt for RouterBuilder {
    fn shard(self, shard: impl Shard) -> Self {
        self.route(ShardRoute::new(shard))
    }

    #[cfg(feature = "discover")]
    fn discover_shards(mut self) -> Self {
        for &shard in inventory::iter::<&'static dyn Shard>() {
            self = self.shard(shard);
        }
        self
    }
}
