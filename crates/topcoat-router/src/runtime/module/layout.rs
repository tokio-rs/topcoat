use topcoat_core::runtime::context::Cx;

use crate::runtime::{Layout, LayoutRenderFuture, Path, PathBuf, Slot};

/// A layout discovered by the module router, produced by the `#[layout]` macro.
///
/// Carries the module path (used to derive the URL prefix from the module
/// tree) and the render function. The module router wraps it in a
/// [`LayoutFromModule`] once the URL path has been resolved.
pub trait ModuleLayout: std::fmt::Debug + Send + Sync + 'static {
    fn module_path(&self) -> &'static str;
    fn render<'a>(&self, cx: &'a Cx, slot: Slot<'a>) -> LayoutRenderFuture<'a>;
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn ModuleLayout);

/// Adapts a [`ModuleLayout`] into a [`Layout`] with a resolved URL path.
#[derive(Debug)]
pub struct LayoutFromModule {
    layout: &'static dyn ModuleLayout,
    path: PathBuf,
}

impl LayoutFromModule {
    pub fn new(layout: &'static dyn ModuleLayout, path: PathBuf) -> Self {
        Self { layout, path }
    }
}

impl Layout for LayoutFromModule {
    fn path(&self) -> &Path {
        &self.path
    }

    fn render<'a>(&self, cx: &'a Cx, slot: Slot<'a>) -> LayoutRenderFuture<'a> {
        self.layout.render(cx, slot)
    }
}
