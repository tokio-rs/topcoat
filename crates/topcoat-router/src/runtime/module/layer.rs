use std::borrow::Cow;

use crate::runtime::{LayerFn, LayerHandlerFn, Path};

/// A layer discovered by the module router, produced by the `#[layer]` macro.
///
/// Holds the module path (for deriving the URL prefix from the module tree)
/// and the handler function. The module router converts each `ModuleLayerFn`
/// into a [`LayerFn`] once the URL path has been computed.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ModuleLayerFn {
    /// Module path where `#[layer]` was declared, used to derive the URL path.
    module_path: &'static str,
    /// The layer's handler function, wrapping the inner chain.
    render: LayerHandlerFn,
}

impl ModuleLayerFn {
    /// Creates a new module layer. Called by the expanded `#[layer]` macro.
    pub const fn new(module_path: &'static str, render: LayerHandlerFn) -> Self {
        Self {
            module_path,
            render,
        }
    }

    /// Converts into a [`LayerFn`] with the given resolved URL path.
    #[must_use]
    pub fn into_layer(self, path: Cow<'static, Path>) -> LayerFn {
        LayerFn::new(path, self.render)
    }

    /// Returns the module path used to derive the URL.
    #[must_use]
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ModuleLayerFn);
