use std::{borrow::Cow, collections::HashMap, pin::Pin};

use http::Method;
use topcoat_core::{context::Cx, error::Result};

use crate::{Body, Path, Response};

/// The async handler function backing a [`Route`].
pub type RouteHandlerFn =
    for<'cx> fn(
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

/// A route handler that handles an HTTP API call.
///
/// Created either manually via `#[route(GET "/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a [`Router`](crate::Router).
#[derive(Debug, Clone)]
pub struct Route {
    /// The HTTP method this route responds to.
    method: Method,
    /// The URL path this route handles.
    path: Cow<'static, Path>,
    /// The async render function that produces the page [`View`].
    handle: RouteHandlerFn,
}

impl Route {
    /// Creates a new route with an explicit method, path, and handler function.
    pub const fn new(method: Method, path: Cow<'static, Path>, handle: RouteHandlerFn) -> Self {
        Self {
            method,
            path,
            handle,
        }
    }

    /// Returns the HTTP method this route responds to.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Returns the URL path this route handles.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Invokes the route's handler, returning a [`Result`].
    pub fn handle<'cx>(
        &self,
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>> {
        (self.handle)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Route);

/// Registry of [`Route`] declarations.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub(crate) struct Routes {
    routes: HashMap<Cow<'static, Path>, Route>,
}

impl Routes {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Default::default()
    }

    /// Registers a route for a router path. Panics on duplicates.
    pub fn register(&mut self, route: Route) {
        if let Some(existing) = self.routes.insert(route.path.clone(), route) {
            panic!("multiple routes registered for path `{}`", existing.path)
        }
    }

    /// Returns `true` if no route has been registered.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

impl IntoIterator for Routes {
    type Item = Route;
    type IntoIter = std::collections::hash_map::IntoValues<Cow<'static, Path>, Route>;

    fn into_iter(self) -> Self::IntoIter {
        self.routes.into_values()
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;

    use super::*;
    use crate::IntoResponse;

    fn dummy_render(
        _cx: &Cx,
        _body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<Response>> + Send>> {
        Box::pin(async { (StatusCode::OK).into_response() })
    }

    fn route(path: &'static str) -> Route {
        Route::new(Method::GET, Cow::Borrowed(Path::new(path)), dummy_render)
    }

    // ── Route ──

    #[test]
    fn route_path() {
        let r = route("/api/health");
        assert_eq!(r.path(), Path::new("/api/health"));
    }

    // ── Routes ──

    #[test]
    fn routes_new_is_empty() {
        let routes = Routes::new();
        assert!(routes.is_empty());
    }

    #[test]
    fn routes_register() {
        let mut routes = Routes::new();
        routes.register(route("/api/health"));
        assert!(!routes.is_empty());
    }

    #[test]
    #[should_panic(expected = "multiple routes registered for path")]
    fn routes_register_duplicate_panics() {
        let mut routes = Routes::new();
        routes.register(route("/api/health"));
        routes.register(route("/api/health"));
    }

    #[test]
    fn routes_into_iter() {
        let mut routes = Routes::new();
        routes.register(route("/api/health"));
        routes.register(route("/api/version"));

        let collected: Vec<_> = routes.into_iter().collect();
        assert_eq!(collected.len(), 2);
    }
}
