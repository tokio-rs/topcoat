use std::{hash::Hash, pin::Pin};

use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    Body, Method, Methods, Path, PathBuf, Route, RouteFuture, RouteId, RouterBuilder,
    response::IntoResponse,
};
use topcoat_view::ViewHandle;

pub(crate) const SHARD_ROUTE_PREFIX: &str = "/_topcoat/shards";

/// The identity of a shard, stable across the server and the client runtime.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShardId(&'static str);

impl ShardId {
    #[must_use]
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0
    }
}

/// The future returned by [`Shard::render`]: a boxed, `Send` future borrowing
/// the shard and its request context.
pub type ShardFuture<'cx> = Pin<Box<dyn Future<Output = Result<ViewHandle>> + Send + 'cx>>;

/// A component that re-renders on the server when its runtime expression
/// arguments change.
///
/// Registered into a [`RouterBuilder`] with
/// [`shard`](RouterBuilderShardExt::shard), which serves it as a route
/// dispatched by [`ShardId`].
pub trait Shard: Send + Sync + 'static {
    /// The identity of this shard.
    fn id(&self) -> ShardId;

    /// Renders the shard for an endpoint request, deserializing its arguments
    /// from `body`.
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
pub struct ShardRoute {
    id: RouteId,
    path: PathBuf,
    shard: Box<dyn Shard>,
}

impl ShardRoute {
    /// Builds the route that serves a shard.
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
    /// Mounts a shard route.
    #[must_use]
    fn shard(self, shard: impl Shard) -> Self;

    /// Registers every shard linked into the binary.
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
