use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    pin::Pin,
};

use topcoat_core::runtime::{context::Cx, error::Error};

use topcoat_view::runtime::View;

use crate::runtime::{EncodedSignals, Signals};

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

pub type ShardRenderFn<S> =
    for<'cx> fn(
        cx: &'cx Cx,
        signals: S,
    ) -> Pin<Box<dyn Future<Output = Result<View, Error>> + Send + 'cx>>;

pub struct Shard<S> {
    id: ShardId,
    render: ShardRenderFn<S>,
}

impl<S> Shard<S> {
    pub const fn new(id: ShardId, render: ShardRenderFn<S>) -> Self {
        Self { id, render }
    }

    #[must_use]
    pub fn id(&self) -> ShardId {
        self.id
    }

    /// Renders the shard by invoking its render function.
    ///
    /// # Errors
    ///
    /// Propagates any error produced by the render function.
    pub async fn render(&self, cx: &Cx, signals: S) -> Result<View, Error> {
        (self.render)(cx, signals).await
    }
}

type RenderDynShardFut<'cx> = Pin<Box<dyn Future<Output = Result<View, Error>> + Send + 'cx>>;

pub trait DynShard: Send + Sync + 'static {
    fn id(&self) -> ShardId;
    fn dyn_render<'cx>(
        &'static self,
        cx: &'cx Cx,
        signals: EncodedSignals,
    ) -> RenderDynShardFut<'cx>;
}

impl<S> DynShard for Shard<S>
where
    S: Signals + Send + Sync + 'static,
{
    fn id(&self) -> ShardId {
        self.id
    }

    fn dyn_render<'cx>(
        &'static self,
        cx: &'cx Cx,
        signals: EncodedSignals,
    ) -> Pin<Box<dyn Future<Output = Result<View, Error>> + Send + 'cx>> {
        Box::pin(async move { (self.render)(cx, S::decode(signals)).await })
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn DynShard);

#[derive(Clone, Default)]
pub struct Shards {
    shards: HashSet<DynShardPtr>,
}

impl Shards {
    #[must_use]
    pub fn new() -> Self {
        Shards::default()
    }

    pub fn register(&mut self, shard: &'static dyn DynShard) {
        self.shards.insert(DynShardPtr(shard));
    }

    /// Returns `true` if no shard has been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.shards.is_empty()
    }
}

impl IntoIterator for Shards {
    type Item = &'static dyn DynShard;
    type IntoIter = std::iter::Map<
        std::collections::hash_set::IntoIter<DynShardPtr>,
        fn(DynShardPtr) -> &'static dyn DynShard,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.shards.into_iter().map(|DynShardPtr(i)| i)
    }
}

#[derive(Copy, Clone)]
#[doc(hidden)]
pub struct DynShardPtr(&'static dyn DynShard);

impl PartialEq for DynShardPtr {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self.0, other.0)
    }
}

impl Eq for DynShardPtr {}

impl Hash for DynShardPtr {
    fn hash<H: Hasher>(&self, h: &mut H) {
        std::ptr::from_ref::<dyn DynShard>(self.0)
            .cast::<()>()
            .hash(h);
    }
}
