use std::{hash::Hash, pin::Pin};

use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use topcoat_router::{
    Body, IntoResponse, Method, Path, PathBuf, Route, RouteFuture, RouterBuilder,
};
use topcoat_view::View;

pub(crate) const SHARD_ROUTE_PREFIX: &str = "/_topcoat/shards";

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

pub type ShardRenderFn =
    for<'cx> fn(
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<View, Error>> + Send + 'cx>>;

#[derive(Debug, Clone)]
pub struct ErasedShard {
    id: ShardId,
    render: ShardRenderFn,
}

impl ErasedShard {
    #[must_use]
    pub const fn new(id: ShardId, render: ShardRenderFn) -> Self {
        Self { id, render }
    }

    #[must_use]
    pub fn id(&self) -> ShardId {
        self.id
    }

    /// Renders the shard for an endpoint request, deserializing its arguments
    /// from `body`.
    ///
    /// # Errors
    ///
    /// Propagates any error returned by the shard's render function, such as a
    /// failure to deserialize the request body.
    #[inline]
    pub async fn render(&self, cx: &Cx, body: Body) -> Result<View> {
        (self.render)(cx, body).await
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ErasedShard);

pub struct ShardRoute {
    path: PathBuf,
    shard: ErasedShard,
}

impl ShardRoute {
    /// Builds the route that serves a shard.
    pub fn new(shard: impl Into<ErasedShard>) -> Self {
        let shard = shard.into();
        Self {
            path: Path::new(&format!("{SHARD_ROUTE_PREFIX}/{}", shard.id().as_str())).to_owned(),
            shard,
        }
    }
}

impl Route for ShardRoute {
    fn method(&self) -> Method {
        // Avoids URL length limits for large parameters.
        Method::POST
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            let view = (self.shard.render)(cx, body).await?;
            view.into_response(cx)
        })
    }
}

/// Registers shards on a [`RouterBuilder`].
pub trait RouterBuilderShardExt {
    /// Mounts a shard route.
    #[must_use]
    fn shard(self, shard: impl Into<ErasedShard>) -> Self;

    /// Registers every shard linked into the binary.
    #[cfg(feature = "discover")]
    #[cfg_attr(docsrs, doc(cfg(feature = "discover")))]
    #[must_use]
    fn discover_shards(self) -> Self;
}

impl RouterBuilderShardExt for RouterBuilder {
    fn shard(self, shard: impl Into<ErasedShard>) -> Self {
        self.route(ShardRoute::new(shard))
    }

    #[cfg(feature = "discover")]
    fn discover_shards(mut self) -> Self {
        for shard in inventory::iter::<ErasedShard>().cloned() {
            self = self.shard(shard);
        }
        self
    }
}
