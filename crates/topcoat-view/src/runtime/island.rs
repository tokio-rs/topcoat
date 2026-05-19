use std::{any::Any, pin::Pin};

use topcoat_core::context::Cx;

use crate::runtime::{EncodedSignals, Signals, View};

pub type IslandRenderFn<S, E> =
    for<'cx> fn(
        cx: &'cx Cx,
        signals: &S,
    ) -> Pin<Box<dyn Future<Output = Result<View, E>> + Send + 'cx>>;

pub struct Island<S, E> {
    render: IslandRenderFn<S, E>,
}

impl<S, E> Island<S, E> {
    pub const fn new(render: IslandRenderFn<S, E>) -> Self {
        Self { render }
    }

    pub async fn render(&self, cx: &Cx, signals: &S) -> Result<View, E> {
        (self.render)(cx, signals).await
    }
}

pub trait DynIsland: Send + Sync + 'static {
    fn dyn_render<'cx>(
        &'static self,
        cx: &'cx Cx,
        signals: EncodedSignals,
    ) -> Pin<Box<dyn Future<Output = Result<View, Box<dyn Any + Send + Sync>>> + Send + 'cx>>;
}

impl<S, E> DynIsland for Island<S, E>
where
    S: Signals + Send + Sync + 'static,
    E: Send + Sync + 'static,
{
    fn dyn_render<'cx>(
        &'static self,
        cx: &'cx Cx,
        signals: EncodedSignals,
    ) -> Pin<Box<dyn Future<Output = Result<View, Box<dyn Any + Send + Sync>>> + Send + 'cx>> {
        Box::pin(async move {
            (self.render)(cx, &S::decode(signals))
                .await
                .map_err(|e| Box::new(e) as Box<dyn Any + Send + Sync>)
        })
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn DynIsland);
