use std::{borrow::Cow, panic::Location};

use crate::{LayerFn, LayerHandlerFn, Path};

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
    /// Where the layer was declared.
    source_location: &'static Location<'static>,
}

impl ModuleLayerFn {
    /// Creates a new module layer. Called by the expanded `#[layer]` macro.
    #[track_caller]
    pub const fn new(module_path: &'static str, render: LayerHandlerFn) -> Self {
        Self {
            module_path,
            render,
            source_location: Location::caller(),
        }
    }

    /// Converts into a [`LayerFn`] with the given resolved URL path.
    #[must_use]
    pub fn into_layer(self, path: Cow<'static, Path>) -> LayerFn {
        LayerFn::with_source_location(path, self.render, self.source_location)
    }

    /// Returns the module path used to derive the URL.
    #[must_use]
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ModuleLayerFn);
