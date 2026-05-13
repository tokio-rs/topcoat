use std::{borrow::Cow, collections::HashMap, pin::Pin};

use topcoat_core::context::Cx;

use crate::{Body, Path, Result};

/// The async render function backing a [`Page`].
pub type PageRenderFn =
    for<'cx> fn(cx: &'cx Cx, body: Body) -> Pin<Box<dyn Future<Output = Result> + Send + 'cx>>;

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
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'cx>> {
        (self.render)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Page);

/// Registry of [`Page`] declarations, keyed by router path.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub(crate) struct Pages {
    pages: HashMap<Cow<'static, Path>, Page>,
}

impl Pages {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Default::default()
    }

    /// Registers a page for a router path. Panics on duplicates.
    pub fn register(&mut self, page: Page) {
        if let Some(existing) = self.pages.insert(page.path.clone(), page) {
            panic!("multiple pages registered for path `{}`", existing.path)
        }
    }

    /// Returns `true` if no page has been registered.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }
}

impl IntoIterator for Pages {
    type Item = Page;
    type IntoIter = std::collections::hash_map::IntoValues<Cow<'static, Path>, Page>;

    fn into_iter(self) -> Self::IntoIter {
        self.pages.into_values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(path: &'static str) -> Page {
        Page::new(Cow::Borrowed(Path::new(path)), |_, _| unimplemented!())
    }

    // ── Page ──

    #[test]
    fn page_path() {
        let p = page("/settings");
        assert_eq!(p.path(), Path::new("/settings"));
    }

    // ── Pages ──

    #[test]
    fn pages_new_is_empty() {
        let pages = Pages::new();
        assert!(pages.is_empty());
    }

    #[test]
    fn pages_register() {
        let mut pages = Pages::new();
        pages.register(page("/"));
        assert!(!pages.is_empty());
    }

    #[test]
    #[should_panic(expected = "multiple pages registered for path")]
    fn pages_register_duplicate_panics() {
        let mut pages = Pages::new();
        pages.register(page("/settings"));
        pages.register(page("/settings"));
    }

    #[test]
    fn pages_into_iter() {
        let mut pages = Pages::new();
        pages.register(page("/"));
        pages.register(page("/about"));

        let collected: Vec<_> = pages.into_iter().collect();
        assert_eq!(collected.len(), 2);
    }
}
