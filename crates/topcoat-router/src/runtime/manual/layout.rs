use std::{pin::Pin, sync::Arc};

use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_view::runtime::View;

use crate::runtime::Path;

/// A future that resolves to the inner page (or nested layout) [`Result`].
///
/// Every layout render function receives a `Slot` and `.await`s it to embed
/// the child content at the desired location.
pub type Slot<'a> = Pin<Box<dyn Future<Output = Result<View>> + Send + 'a>>;

/// The future returned by [`Layout::render`].
pub type LayoutRenderFuture<'a> = Pin<Box<dyn Future<Output = Result<View>> + Send + 'a>>;

pub trait Layout: std::fmt::Debug + Send + Sync + 'static {
    fn path(&self) -> &Path;
    fn render<'a>(&self, cx: &'a Cx, slot: Slot<'a>) -> LayoutRenderFuture<'a>;
}

impl<L> Layout for &'static L
where
    L: Layout + ?Sized,
{
    #[inline]
    fn path(&self) -> &Path {
        (*self).path()
    }

    #[inline]
    fn render<'a>(&self, cx: &'a Cx, slot: Slot<'a>) -> LayoutRenderFuture<'a> {
        (*self).render(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Layout);

/// Shared registry of layouts, prefix-matched against page paths.
pub type Layouts = Vec<Arc<dyn Layout>>;
