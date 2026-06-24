use std::borrow::Cow;

use http::Method;

use crate::runtime::{Path, RouteFn, RouteHandlerFn};

/// A route discovered by the module router, produced by the `#[route]` macro.
///
/// Holds the module path (for deriving the URL path from the module tree)
/// and the render function. The module router converts each `ModuleRouteFn`
/// into a [`RouteFn`] once the URL path has been computed.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ModuleRouteFn {
    /// The HTTP method triggering this route.
    method: Method,
    /// Module path where `#[route]` was declared, used to derive the URL path.
    module_path: &'static str,
    /// The route's async handler function, returning a [`Result`].
    pub(super) render: RouteHandlerFn,
}

impl ModuleRouteFn {
    /// Creates a new module route. Called by the expanded `#[route]` macro.
    pub const fn new(method: Method, module_path: &'static str, render: RouteHandlerFn) -> Self {
        Self {
            method,
            module_path,
            render,
        }
    }

    /// Converts into a [`RouteFn`] with the given resolved URL path.
    #[must_use]
    pub fn into_route(self, path: Cow<'static, Path>) -> RouteFn {
        RouteFn::new(self.method, path, self.render)
    }

    /// Returns the module path used to derive the URL.
    #[must_use]
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ModuleRouteFn);
