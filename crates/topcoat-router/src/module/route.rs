use std::{borrow::Cow, panic::Location};

use crate::{OwnedMethods, Path, RouteFn, RouteHandlerFn};

/// A route discovered by the module router, produced by the `#[route]` macro.
///
/// Holds the module path (for deriving the URL path from the module tree)
/// and the render function. The module router converts each `ModuleRouteFn`
/// into a [`RouteFn`] once the URL path has been computed.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ModuleRouteFn {
    /// The HTTP methods triggering this route.
    methods: OwnedMethods,
    /// Module path where `#[route]` was declared, used to derive the URL path.
    module_path: &'static str,
    /// The route's async handler function, returning a [`Result`].
    pub(super) render: RouteHandlerFn,
    /// Where the route was declared.
    source_location: &'static Location<'static>,
}

impl ModuleRouteFn {
    /// Creates a new module route. Called by the expanded `#[route]` macro.
    #[track_caller]
    pub const fn new(
        methods: OwnedMethods,
        module_path: &'static str,
        render: RouteHandlerFn,
    ) -> Self {
        Self {
            methods,
            module_path,
            render,
            source_location: Location::caller(),
        }
    }

    /// Converts into a [`RouteFn`] with the given resolved URL path.
    #[must_use]
    pub fn into_route(self, path: Cow<'static, Path>) -> RouteFn {
        RouteFn::with_source_location(self.methods, path, self.render, self.source_location)
    }

    /// Returns the module path used to derive the URL.
    #[must_use]
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ModuleRouteFn);
