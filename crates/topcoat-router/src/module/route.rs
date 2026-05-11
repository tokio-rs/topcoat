use std::borrow::Cow;

use crate::{Path, Route, RouteHandlerFn};

/// A route discovered by the module router, produced by the `#[route]` macro.
///
/// Holds the module path (for deriving the URL path from the module tree)
/// and the render function. The module router converts each `ModuleRoute` into
/// a [`Route`] once the URL path has been computed.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ModuleRoute {
    /// Module path where `#[route]` was declared, used to derive the URL path.
    module_path: &'static str,
    /// The route's async handler function, returning a [`Result`].
    pub(super) render: RouteHandlerFn,
}

impl ModuleRoute {
    /// Creates a new module route. Called by the expanded `#[route]` macro.
    pub const fn new(module_path: &'static str, render: RouteHandlerFn) -> Self {
        Self {
            module_path,
            render,
        }
    }

    /// Converts into a [`Route`] with the given resolved URL path.
    pub fn into_route(self, path: Cow<'static, Path>) -> Route {
        Route::new(path, self.render)
    }

    /// Returns the module path used to derive the URL.
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ModuleRoute);
