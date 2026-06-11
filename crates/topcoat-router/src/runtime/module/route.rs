use http::Method;
use topcoat_core::runtime::context::Cx;

use crate::runtime::{Body, Path, PathBuf, Route, RouteHandlerFuture};

/// A route discovered by the module router, produced by the `#[route]` macro.
///
/// Carries the module path (used to derive the URL from the module tree) and
/// the handler. The module router wraps it in a [`RouteFromModule`] once the
/// URL path has been resolved.
pub trait ModuleRoute: Send + Sync + 'static {
    fn method(&self) -> Method;
    fn module_path(&self) -> &'static str;
    fn handle<'a>(&'a self, cx: &'a Cx, body: Body) -> RouteHandlerFuture<'a>;
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn ModuleRoute);

/// Adapts a [`ModuleRoute`] into a [`Route`] with a resolved URL path.
#[derive(Clone)]
pub struct RouteFromModule {
    route: &'static dyn ModuleRoute,
    path: PathBuf,
}

impl RouteFromModule {
    pub fn new(route: &'static dyn ModuleRoute, path: PathBuf) -> Self {
        Self { route, path }
    }
}

impl Route for RouteFromModule {
    fn method(&self) -> Method {
        self.route.method()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body) -> RouteHandlerFuture<'a> {
        self.route.handle(cx, body)
    }
}
