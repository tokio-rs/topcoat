use std::{
    any::Any,
    collections::HashSet,
    hash::{Hash, Hasher},
    pin::Pin,
};

use topcoat_core::context::Cx;

use topcoat_view::runtime::View;

use crate::runtime::{EncodedSignals, Signals};

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ShardId(&'static str);

impl ShardId {
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    pub fn as_str(&self) -> &str {
        self.0
    }
}

pub type ShardRenderFn<S, E> =
    for<'cx> fn(
        cx: &'cx Cx,
        signals: S,
    ) -> Pin<Box<dyn Future<Output = Result<View, E>> + Send + 'cx>>;

pub struct Shard<S, E> {
    id: ShardId,
    render: ShardRenderFn<S, E>,
}

impl<S, E> Shard<S, E> {
    pub const fn new(id: ShardId, render: ShardRenderFn<S, E>) -> Self {
        Self { id, render }
    }

    pub fn id(&self) -> ShardId {
        self.id
    }

    pub async fn render(&self, cx: &Cx, signals: S) -> Result<View, E> {
        (self.render)(cx, signals).await
    }
}

type RenderDynShardFut<'cx> =
    Pin<Box<dyn Future<Output = Result<View, Box<dyn Any + Send + Sync>>> + Send + 'cx>>;

pub trait DynShard: Send + Sync + 'static {
    fn id(&self) -> ShardId;
    fn dyn_render<'cx>(
        &'static self,
        cx: &'cx Cx,
        signals: EncodedSignals,
    ) -> RenderDynShardFut<'cx>;
}

impl<S, E> DynShard for Shard<S, E>
where
    S: Signals + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn id(&self) -> ShardId {
        self.id
    }

    fn dyn_render<'cx>(
        &'static self,
        cx: &'cx Cx,
        signals: EncodedSignals,
    ) -> Pin<Box<dyn Future<Output = Result<View, Box<dyn Any + Send + Sync>>> + Send + 'cx>> {
        Box::pin(async move {
            (self.render)(cx, S::decode(signals))
                .await
                .map_err(|e| Box::new(e) as Box<dyn Any + Send + Sync>)
        })
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn DynShard);

#[derive(Clone, Default)]
pub struct Shards {
    shards: HashSet<DynShardPtr>,
}

impl Shards {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register(&mut self, shard: &'static dyn DynShard) {
        self.shards.insert(DynShardPtr(shard));
    }

    /// Returns `true` if no shard has been registered.
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
        (self.0 as *const dyn DynShard).cast::<()>().hash(h);
    }
}
