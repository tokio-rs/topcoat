use topcoat_core::runtime::context::Cx;

use crate::runtime::{Body, Page, PageRenderFuture, Path, PathBuf};

/// A page discovered by the module router, produced by the `#[page]` macro.
///
/// Carries the module path (used to derive the URL from the module tree) and
/// the render function. The module router wraps it in a [`PageFromModule`]
/// once the URL path has been resolved.
pub trait ModulePage: std::fmt::Debug + Send + Sync + 'static {
    fn module_path(&self) -> &'static str;
    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> PageRenderFuture<'a>;
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn ModulePage);

/// Adapts a [`ModulePage`] into a [`Page`] with a resolved URL path.
#[derive(Debug)]
pub struct PageFromModule {
    page: &'static dyn ModulePage,
    path: PathBuf,
}

impl PageFromModule {
    pub fn new(page: &'static dyn ModulePage, path: PathBuf) -> Self {
        Self { page, path }
    }
}

impl Page for PageFromModule {
    fn path(&self) -> &Path {
        &self.path
    }

    fn render<'a>(&'a self, cx: &'a Cx, body: Body) -> PageRenderFuture<'a> {
        self.page.render(cx, body)
    }
}
