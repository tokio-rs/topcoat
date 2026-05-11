use std::{borrow::Cow, collections::HashMap, pin::Pin};

use topcoat_core::context::Cx;

use crate::{Path, Result};

/// The async render function backing a [`Layout`], receiving a [`Slot`] for child content.
pub type LayoutRenderFn =
    for<'cx> fn(cx: &'cx Cx, slot: Slot<'cx>) -> Pin<Box<dyn Future<Output = Result> + Send + 'cx>>;

/// A future that resolves to the inner page (or nested layout) [`Result`].
///
/// Every layout render function receives a `Slot` and `.await`s it to embed
/// the child content at the desired location.
pub type Slot<'cx> = Pin<Box<dyn Future<Output = Result> + Send + 'cx>>;

/// A layout that wraps pages whose path starts with the layout's path prefix.
///
/// When multiple layouts match a page, they nest from most-specific (innermost)
/// to least-specific (outermost). For example, layouts at `/` and `/settings`
/// both match `/settings/profile`, rendering as: root → settings → page.
#[derive(Debug, Clone)]
pub struct Layout {
    /// The path prefix this layout applies to.
    path: Cow<'static, Path>,
    /// The async render function that wraps child content via a [`Slot`].
    render: LayoutRenderFn,
}

impl Layout {
    /// Creates a new layout with an explicit path and render function.
    pub const fn new(path: Cow<'static, Path>, render: LayoutRenderFn) -> Self {
        Self { path, render }
    }

    /// Returns the path prefix this layout applies to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Renders the layout, embedding the given [`Slot`] as child content.
    pub fn render<'cx>(
        &self,
        cx: &'cx Cx,
        slot: Slot<'cx>,
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'cx>> {
        (self.render)(cx, slot)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Layout);

/// Registry of [`Layout`] declarations, keyed by router path.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub(crate) struct Layouts {
    layouts: HashMap<Cow<'static, Path>, Layout>,
}

impl Layouts {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Default::default()
    }

    /// Registers a layout for a router path. Panics on duplicates.
    pub fn register(&mut self, layout: Layout) {
        if let Some(existing) = self.layouts.insert(layout.path.clone(), layout) {
            panic!("multiple layouts registered for path `{}`", existing.path)
        }
    }

    /// Returns an iterator over all layouts whose path prefix matches the given path.
    pub fn for_path(&self, path: &Path) -> impl Iterator<Item = &Layout> {
        self.layouts
            .values()
            .filter(|layout| path.starts_with(layout.path()))
    }

    /// Returns `true` if no layout has been registered.
    pub fn is_empty(&self) -> bool {
        self.layouts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_render<'cx>(
        _cx: &'cx Cx,
        slot: Slot<'cx>,
    ) -> Pin<Box<dyn Future<Output = Result> + Send + 'cx>> {
        Box::pin(slot)
    }

    fn layout(path: &'static str) -> Layout {
        Layout::new(Cow::Borrowed(Path::new(path)), dummy_render)
    }

    // ── Layout ──

    #[test]
    fn layout_path() {
        let l = layout("/settings");
        assert_eq!(l.path(), Path::new("/settings"));
    }

    // ── Layouts ──

    #[test]
    fn layouts_new_is_empty() {
        let layouts = Layouts::new();
        assert!(layouts.is_empty());
    }

    #[test]
    fn layouts_register() {
        let mut layouts = Layouts::new();
        layouts.register(layout("/"));
        assert!(!layouts.is_empty());
    }

    #[test]
    #[should_panic(expected = "multiple layouts registered for path")]
    fn layouts_register_duplicate_panics() {
        let mut layouts = Layouts::new();
        layouts.register(layout("/settings"));
        layouts.register(layout("/settings"));
    }

    #[test]
    fn layouts_for_path_root_matches_all() {
        let mut layouts = Layouts::new();
        layouts.register(layout("/"));
        layouts.register(layout("/settings"));

        let matched: Vec<_> = layouts.for_path(Path::new("/settings/profile")).collect();
        assert_eq!(matched.len(), 2);
    }

    #[test]
    fn layouts_for_path_filters_non_matching() {
        let mut layouts = Layouts::new();
        layouts.register(layout("/settings"));
        layouts.register(layout("/admin"));

        let matched: Vec<_> = layouts.for_path(Path::new("/settings/profile")).collect();
        assert_eq!(matched.len(), 1);
        assert_eq!(matched[0].path(), Path::new("/settings"));
    }

    #[test]
    fn layouts_for_path_no_match() {
        let mut layouts = Layouts::new();
        layouts.register(layout("/admin"));

        let matched: Vec<_> = layouts.for_path(Path::new("/settings")).collect();
        assert!(matched.is_empty());
    }
}
