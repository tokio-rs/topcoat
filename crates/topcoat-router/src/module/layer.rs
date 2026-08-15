use topcoat_core::context::Cx;

use crate::{Body, Layer, LayerFuture, Next, Path, PathBuf};

/// A layer discovered by the module router, declared without an explicit path.
///
/// Holds the module path the layer was declared in; the module router derives
/// the URL prefix from the module tree and registers the layer under it.
#[doc(hidden)]
pub trait ModuleLayer: Send + Sync + 'static {
    /// The module path where the layer was declared, used to derive the URL.
    fn module_path(&self) -> &'static str;

    /// Handles a request, calling `next` to continue down the chain.
    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a>;
}

impl<L: ModuleLayer + ?Sized> ModuleLayer for &'static L {
    fn module_path(&self) -> &'static str {
        (**self).module_path()
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        (**self).handle(cx, body, next)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn ModuleLayer);

/// A [`ModuleLayer`] bound to the URL prefix derived from its module tree,
/// registered into the inner builder as a [`Layer`].
pub(super) struct ResolvedLayer<L> {
    layer: L,
    path: PathBuf,
}

impl<L: ModuleLayer> ResolvedLayer<L> {
    pub(super) fn new(layer: L, path: PathBuf) -> Self {
        Self { layer, path }
    }
}

impl<L: ModuleLayer> Layer for ResolvedLayer<L> {
    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        self.layer.handle(cx, body, next)
    }
}
