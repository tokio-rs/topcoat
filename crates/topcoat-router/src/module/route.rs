use topcoat_core::context::Cx;

use crate::{Body, Methods, Path, PathBuf, Route, RouteFuture, RouteId};

/// A route discovered by the module router, declared without an explicit path.
///
/// Holds the module path the route was declared in; the module router derives
/// the URL path from the module tree and registers the route under it.
#[doc(hidden)]
pub trait ModuleRoute: Send + Sync + 'static {
    /// The identity of this route's handler.
    fn id(&self) -> RouteId;

    /// The HTTP methods this route responds to.
    fn methods(&self) -> Methods<'_>;

    /// The module path where the route was declared, used to derive the URL.
    fn module_path(&self) -> &'static str;

    /// Handles a request, producing a response.
    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;
}

impl<R: ModuleRoute + ?Sized> ModuleRoute for &'static R {
    fn id(&self) -> RouteId {
        (**self).id()
    }

    fn methods(&self) -> Methods<'_> {
        (**self).methods()
    }

    fn module_path(&self) -> &'static str {
        (**self).module_path()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (**self).handle(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn ModuleRoute);

/// A [`ModuleRoute`] bound to the URL path derived from its module tree,
/// registered into the inner builder as a [`Route`].
pub(super) struct ResolvedRoute<R> {
    route: R,
    path: PathBuf,
}

impl<R: ModuleRoute> ResolvedRoute<R> {
    pub(super) fn new(route: R, path: PathBuf) -> Self {
        Self { route, path }
    }
}

impl<R: ModuleRoute> Route for ResolvedRoute<R> {
    fn id(&self) -> RouteId {
        self.route.id()
    }

    fn methods(&self) -> Methods<'_> {
        self.route.methods()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        self.route.handle(cx, body)
    }
}
