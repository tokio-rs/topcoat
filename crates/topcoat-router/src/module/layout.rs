use std::{borrow::Cow, pin::Pin};

use crate::{Layout, Path, Result, Slot};

/// A layout discovered by the module router, produced by the `#[layout]` macro.
///
/// Holds the module path (for deriving the URL prefix from the module tree)
/// and the render function. The module router converts each `ModuleLayout`
/// into a [`Layout`] once the URL path has been computed.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ModuleLayout {
    /// Module path where `#[layout]` was declared, used to derive the URL path.
    module_path: &'static str,
    /// The layout's async render function, receiving a [`Slot`] and returning a [`Result`].
    render: fn(slot: Slot) -> Pin<Box<dyn Future<Output = Result> + Send>>,
}

impl ModuleLayout {
    /// Creates a new module layout. Called by the expanded `#[layout]` macro.
    pub const fn new(
        module_path: &'static str,
        render: fn(slot: Slot) -> Pin<Box<dyn Future<Output = Result> + Send>>,
    ) -> Self {
        Self {
            module_path,
            render,
        }
    }

    /// Converts into a [`Layout`] with the given resolved URL path.
    pub fn into_layout(self, path: Cow<'static, Path>) -> Layout {
        Layout::new(path, self.render)
    }

    /// Returns the module path used to derive the URL.
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ModuleLayout);
