use std::{borrow::Cow, pin::Pin};

use http::Method;
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, Path, Response};

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
