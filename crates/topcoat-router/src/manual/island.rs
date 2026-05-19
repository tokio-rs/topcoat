use std::pin::Pin;

use topcoat_core::context::Cx;

use crate::runtime::View;

pub type IslandRenderFn<E> =
    for<'cx> fn(cx: &'cx Cx) -> Pin<Box<dyn Future<Output = Result<View, E>> + Send + 'cx>>;

pub struct Island<E> {
    render: IslandRenderFn<E>,
}

impl<E> Island<E> {
    pub const fn new(render: IslandRenderFn<E>) -> Self {
        Self { render }
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Island);
