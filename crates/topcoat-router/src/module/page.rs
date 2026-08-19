use topcoat_core::{context::Cx, error::Result};
use topcoat_view::View;

use crate::{Body, Layout, Methods, Page, Path, PathBuf, RouteId, ViewFuture, route};

/// A page discovered by the module router, declared without an explicit path.
///
/// Holds the module path the page was declared in; the module router derives
/// the URL path from the module tree and registers the page under it.
#[doc(hidden)]
pub trait ModulePage: Send + Sync + 'static {
    /// The identity of this page's handler.
    fn id(&self) -> RouteId;

    /// The HTTP methods this page responds to.
    fn methods(&self) -> Methods<'_>;

    /// The module path where the page was declared, used to derive the URL.
    fn module_path(&self) -> &'static str;

    /// Renders the page [`View`].
    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> ViewFuture<'cx>;

    /// Returns whether this page handles the current request.
    ///
    /// Only the handler is compared, so a page is current for every value its
    /// path parameters take, whatever the request's query or fragment.
    ///
    /// # Panics
    ///
    /// Panics if the request matched no route: either its path matched no
    /// endpoint, or the endpoint holds no route for the request's method.
    fn is_current(&self, cx: &Cx) -> bool {
        route(cx).id() == self.id()
    }
}

impl<P: ModulePage + ?Sized> ModulePage for &'static P {
    fn id(&self) -> RouteId {
        (**self).id()
    }

    fn methods(&self) -> Methods<'_> {
        (**self).methods()
    }

    fn module_path(&self) -> &'static str {
        (**self).module_path()
    }

    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> ViewFuture<'cx> {
        (**self).render(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn ModulePage);

/// A [`ModulePage`] bound to the URL path derived from its module tree,
/// registered into the inner builder as a [`Page`].
pub(super) struct ResolvedPage<P> {
    page: P,
    path: PathBuf,
}

impl<P: ModulePage> ResolvedPage<P> {
    pub(super) fn new(page: P, path: PathBuf) -> Self {
        Self { page, path }
    }
}

impl<P: ModulePage> Page for ResolvedPage<P> {
    fn id(&self) -> RouteId {
        self.page.id()
    }

    fn methods(&self) -> Methods<'_> {
        self.page.methods()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn render<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> ViewFuture<'cx> {
        self.page.render(cx, body)
    }
}

/// A layout discovered by the module router, declared without an explicit
/// path.
///
/// Holds the module path the layout was declared in; the module router derives
/// the URL prefix from the module tree and registers the layout under it.
#[doc(hidden)]
pub trait ModuleLayout: Send + Sync + 'static {
    /// The module path where the layout was declared, used to derive the URL.
    fn module_path(&self) -> &'static str;

    /// Renders the layout, embedding the given child content
    /// [`Result`]`<`[`View`]`>` as its slot.
    fn render<'cx>(&'cx self, cx: &'cx Cx, slot: Result<View>) -> ViewFuture<'cx>;
}

impl<L: ModuleLayout + ?Sized> ModuleLayout for &'static L {
    fn module_path(&self) -> &'static str {
        (**self).module_path()
    }

    fn render<'cx>(&'cx self, cx: &'cx Cx, slot: Result<View>) -> ViewFuture<'cx> {
        (**self).render(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn ModuleLayout);

/// A [`ModuleLayout`] bound to the URL prefix derived from its module tree,
/// registered into the inner builder as a [`Layout`].
pub(super) struct ResolvedLayout<L> {
    layout: L,
    path: PathBuf,
}

impl<L: ModuleLayout> ResolvedLayout<L> {
    pub(super) fn new(layout: L, path: PathBuf) -> Self {
        Self { layout, path }
    }
}

impl<L: ModuleLayout> Layout for ResolvedLayout<L> {
    fn path(&self) -> &Path {
        &self.path
    }

    fn render<'cx>(&'cx self, cx: &'cx Cx, slot: Result<View>) -> ViewFuture<'cx> {
        self.layout.render(cx, slot)
    }
}
