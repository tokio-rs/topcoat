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
pub struct IslandId(&'static str);

impl IslandId {
    pub const fn new(inner: &'static str) -> Self {
        Self(inner)
    }

    pub fn as_str(&self) -> &str {
        self.0
    }
}

pub type IslandRenderFn<S, E> =
    for<'cx> fn(
        cx: &'cx Cx,
        signals: S,
    ) -> Pin<Box<dyn Future<Output = Result<View, E>> + Send + 'cx>>;

pub struct Island<S, E> {
    id: IslandId,
    render: IslandRenderFn<S, E>,
}

impl<S, E> Island<S, E> {
    pub const fn new(id: IslandId, render: IslandRenderFn<S, E>) -> Self {
        Self { id, render }
    }

    pub fn id(&self) -> IslandId {
        self.id
    }

    pub async fn render(&self, cx: &Cx, signals: S) -> Result<View, E> {
        (self.render)(cx, signals).await
    }
}

type RenderDynIslandFut<'cx> =
    Pin<Box<dyn Future<Output = Result<View, Box<dyn Any + Send + Sync>>> + Send + 'cx>>;

pub trait DynIsland: Send + Sync + 'static {
    fn id(&self) -> IslandId;
    fn dyn_render<'cx>(
        &'static self,
        cx: &'cx Cx,
        signals: EncodedSignals,
    ) -> RenderDynIslandFut<'cx>;
}

impl<S, E> DynIsland for Island<S, E>
where
    S: Signals + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn id(&self) -> IslandId {
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
inventory::collect!(&'static dyn DynIsland);

#[derive(Clone, Default)]
pub struct Islands {
    islands: HashSet<DynIslandPtr>,
}

impl Islands {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn register(&mut self, island: &'static dyn DynIsland) {
        self.islands.insert(DynIslandPtr(island));
    }

    /// Returns `true` if no island has been registered.
    pub fn is_empty(&self) -> bool {
        self.islands.is_empty()
    }
}

impl IntoIterator for Islands {
    type Item = &'static dyn DynIsland;
    type IntoIter = std::iter::Map<
        std::collections::hash_set::IntoIter<DynIslandPtr>,
        fn(DynIslandPtr) -> &'static dyn DynIsland,
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.islands.into_iter().map(|DynIslandPtr(i)| i)
    }
}

#[derive(Copy, Clone)]
#[doc(hidden)]
pub struct DynIslandPtr(&'static dyn DynIsland);

impl PartialEq for DynIslandPtr {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self.0, other.0)
    }
}

impl Eq for DynIslandPtr {}

impl Hash for DynIslandPtr {
    fn hash<H: Hasher>(&self, h: &mut H) {
        (self.0 as *const dyn DynIsland).cast::<()>().hash(h);
    }
}
