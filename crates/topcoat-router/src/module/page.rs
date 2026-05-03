use std::{borrow::Cow, pin::Pin};

use crate::{Page, Path, Result};

/// A page discovered by the module router, produced by the `#[page]` macro.
///
/// Holds the module path (for deriving the URL path from the module tree)
/// and the render function. The module router converts each `ModulePage` into
/// a [`Page`] once the URL path has been computed.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct ModulePage {
    /// Module path where `#[page]` was declared, used to derive the URL path.
    module_path: &'static str,
    /// The page's async render function, returning a [`Result`].
    pub(super) render: fn() -> Pin<Box<dyn Future<Output = Result> + Send>>,
}

impl ModulePage {
    /// Creates a new module page. Called by the expanded `#[page]` macro.
    pub const fn new(
        module_path: &'static str,
        render: fn() -> Pin<Box<dyn Future<Output = Result> + Send>>,
    ) -> Self {
        Self {
            module_path,
            render,
        }
    }

    /// Converts into a [`Page`] with the given resolved URL path.
    pub fn into_page(self, path: Cow<'static, Path>) -> Page {
        Page::new(path, self.render)
    }

    /// Returns the module path used to derive the URL.
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }
}

#[cfg(feature = "discover")]
inventory::collect!(ModulePage);
