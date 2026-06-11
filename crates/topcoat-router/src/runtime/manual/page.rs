use std::{borrow::Cow, pin::Pin};

use topcoat_core::runtime::{context::Cx, error::Result};
use topcoat_view::runtime::View;

use crate::runtime::{Body, Path};

/// The async render function backing a [`Page`].
pub type PageRenderFn = for<'cx> fn(
    cx: &'cx Cx,
    body: Body,
) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>>;

/// A route handler that renders a [`View`] for a specific URL path.
///
/// Created either manually via `#[page("/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a
/// [`Router`](crate::Router) alongside [`Layout`](crate::Layout)s.
#[derive(Debug, Clone)]
pub struct Page {
    /// The URL path this page handles.
    path: Cow<'static, Path>,
    /// The async render function that produces the page [`View`].
    render: PageRenderFn,
}

impl Page {
    /// Creates a new page with an explicit path and render function.
    pub const fn new(path: Cow<'static, Path>, render: PageRenderFn) -> Self {
        Self { path, render }
    }

    /// Returns the URL path this page handles.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Renders the page, returning a [`Result`].
    pub fn render<'cx>(
        &self,
        cx: &'cx Cx,
        body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>> {
        (self.render)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Page);
