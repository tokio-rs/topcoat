use std::{borrow::Cow, pin::Pin};

use http::Method;
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, Path, Response};

/// The future returned by [`Route::handle`]: a boxed, `Send` future borrowing
/// the route and its request context.
pub type RouteFuture<'cx> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

/// A single routable endpoint: an HTTP method, a URL path, and a handler.
///
/// This is the core primitive a [`Router`](crate::runtime::Router) dispatches to.
/// Register any `Route` with [`RouterBuilder::route`](crate::runtime::RouterBuilder::route).
pub trait Route: Send + Sync + 'static {
    /// The HTTP method this route responds to.
    fn method(&self) -> Method;

    /// The URL path this route handles.
    fn path(&self) -> &Path;

    /// Handles a request, producing a response.
    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;
}

/// The async handler function backing a [`RouteFn`].
pub type RouteHandlerFn = for<'cx> fn(cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;

/// A [`Route`] backed by a plain handler function.
///
/// Created either manually via `#[route(GET "/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a
/// [`Router`](crate::runtime::Router).
#[derive(Debug, Clone)]
pub struct RouteFn {
    /// The HTTP method this route responds to.
    method: Method,
    /// The URL path this route handles.
    path: Cow<'static, Path>,
    /// The handler function that produces the response.
    handle: RouteHandlerFn,
}

impl RouteFn {
    /// Creates a new route with an explicit method, path, and handler function.
    pub const fn new(method: Method, path: Cow<'static, Path>, handle: RouteHandlerFn) -> Self {
        Self {
            method,
            path,
            handle,
        }
    }
}

impl Route for RouteFn {
    fn method(&self) -> Method {
        self.method.clone()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (self.handle)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(RouteFn);
