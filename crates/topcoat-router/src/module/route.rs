use std::borrow::Cow;

use crate::{HrefTarget, OwnedMethods, Path, RouteFn, RouteHandlerFn, RouteId, RouteIdCell};

/// A route discovered by the module router, produced by the `#[route]` macro.
///
/// Holds the module path (for deriving the URL path from the module tree)
/// and the render function. The module router converts each `ModuleRouteFn`
/// into a [`RouteFn`] once the URL path has been computed.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ModuleRouteFn {
    /// The identity of this route's handler.
    id: RouteIdCell,
    /// The HTTP methods triggering this route.
    methods: OwnedMethods,
    /// Module path where `#[route]` was declared, used to derive the URL path.
    module_path: &'static str,
    /// The route's async handler function, returning a [`Result`].
    pub(super) render: RouteHandlerFn,
}

impl ModuleRouteFn {
    /// Creates a new module route. Called by the expanded `#[route]` macro.
    pub const fn new(
        methods: OwnedMethods,
        module_path: &'static str,
        render: RouteHandlerFn,
    ) -> Self {
        Self {
            id: RouteIdCell::new(),
            methods,
            module_path,
            render,
        }
    }

    /// Returns the identity of this route's handler.
    pub fn id(&self) -> RouteId {
        self.id.get()
    }

    /// Converts into a [`RouteFn`] with the given resolved URL path.
    #[must_use]
    pub fn into_route(self, path: Cow<'static, Path>) -> RouteFn {
        RouteFn::new(self.methods, path, self.render).with_id(self.id)
    }

    /// Returns the module path used to derive the URL.
    #[must_use]
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static ModuleRouteFn);

impl HrefTarget for ModuleRouteFn {
    fn route_id(&self) -> RouteId {
        self.id.get()
    }
}
