use std::borrow::Cow;

use heck::ToKebabCase;

use super::{
    layer::ResolvedLayer,
    page::{ResolvedLayout, ResolvedPage},
    route::ResolvedRoute,
};
use crate::{
    ModuleLayer, ModuleLayout, ModulePage, ModuleRoute, PathBuf, PathSegment, RouterBuilder,
    Segment, SegmentKind, Segments,
};

/// The module-based router builder, created by the `module_router!` macro.
///
/// Translates Rust module paths into route paths and builds a
/// [`RouterBuilder`]. The module tree rooted at `root_module_path` becomes the
/// route tree: each module maps to a path segment, with `_`-prefixed modules
/// becoming groups and static names kebab-cased.
///
/// Segment overrides (kind, rename) registered via `segment!` are applied during
/// the module-to-path translation. Overrides must be registered before any pages
/// or layouts: this is enforced with a panic.
///
/// The translation pipeline for a given module path:
/// 1. Strip the `root_module_path` prefix
/// 2. Walk each `::`-separated component, checking for a [`Segment`] override
/// 3. Apply default kind (`_` prefix -> `Group`, otherwise `Static`)
/// 4. Kebab-case static segment names, leave others as-is
/// 5. Collect into a [`PathBuf`]
#[doc(hidden)]
pub struct ModuleRouterBuilder {
    inner: RouterBuilder,
    root_module_path: &'static str,
    segments: Segments,
}

impl ModuleRouterBuilder {
    /// Creates a new `ModuleRouterBuilder` rooted at the given module path.
    ///
    /// The `root_module_path` is the `module_path!()` of the module calling
    /// `module_router!`.
    #[must_use]
    pub fn new(root_module_path: &'static str) -> Self {
        Self {
            root_module_path,
            inner: RouterBuilder::new(),
            segments: Segments::new(),
        }
    }

    /// Returns the module path relative to the root.
    ///
    /// This is the key used to look up [`Segment`] overrides and to derive
    /// route path segments.
    #[track_caller]
    fn relative_module_path(&self, module_path: &'static str) -> &'static str {
        if module_path == self.root_module_path {
            return "";
        }
        module_path
            .strip_prefix(self.root_module_path)
            .and_then(|s| s.strip_prefix("::"))
            .expect("module path must be under module router's root")
    }

    /// Registers a [`Segment`] override for a module path.
    ///
    /// # Panics
    ///
    /// Panics if any pages or layouts have already been registered, since
    /// segment overrides affect path computation and must come first.
    #[must_use]
    #[track_caller]
    pub fn segment(mut self, segment: Segment) -> Self {
        assert!(
            self.inner.is_empty(),
            "`segment` must be called before registering any resource"
        );
        self.segments
            .register(self.relative_module_path(segment.module_path()), segment);
        self
    }

    /// Converts a module path to a route [`PathBuf`].
    ///
    /// Walks each `::`-separated component of the relative module path, applying
    /// segment overrides and defaults (kebab-case for static, `_` prefix for
    /// groups) to build the final route path.
    fn module_path_to_path(&self, module_path: &'static str) -> PathBuf {
        let relative = self.relative_module_path(module_path);
        let mut path_buf = PathBuf::new();

        if relative.is_empty() {
            return path_buf;
        }

        // Iterate over the module structure. At each module level, check if there is a matching
        // [`Segment`] for that path specified by the user that overrides the default behavior.
        let mut prefix_end = 0;
        for (i, component) in relative.split("::").enumerate() {
            if i > 0 {
                prefix_end += "::".len();
            }
            prefix_end += component.len();
            let segment = self.segments.get(&relative[..prefix_end]);

            // A module is a group segment if it starts with "_" or a static segment otherwise,
            // unless this is overridden by the user.
            let kind = match segment.and_then(|segment| segment.kind()) {
                Some(kind) => *kind,
                None => {
                    if component.starts_with('_') {
                        SegmentKind::Group
                    } else {
                        SegmentKind::Static
                    }
                }
            };
            // Static segments are converted to kebab-case, other modules names are left as is.
            // This can also be overridden by the user.
            let name = match segment.and_then(|segment| segment.rename()) {
                Some(rename) => Cow::Borrowed(rename),
                None => match kind {
                    SegmentKind::Static => Cow::Owned(component.to_kebab_case()),
                    _ => Cow::Borrowed(component),
                },
            };

            let path_segment = match kind {
                SegmentKind::Static => PathSegment::Static(&name),
                SegmentKind::Group => PathSegment::Group(&name),
                SegmentKind::Param => PathSegment::Param(&name),
                SegmentKind::CatchAll => PathSegment::CatchAll(&name),
            };

            path_buf += path_segment;
        }
        path_buf
    }

    /// Registers a [`ModulePage`], computing its route path from the module path.
    ///
    /// # Panics
    ///
    /// Panics if a page has already been registered for the same path.
    #[must_use]
    pub fn page(mut self, page: impl ModulePage) -> Self {
        let path = self.module_path_to_path(page.module_path());
        self.inner = self.inner.page(ResolvedPage::new(page, path));
        self
    }

    /// Registers a [`ModuleLayout`], computing its route path from the module path.
    ///
    /// # Panics
    ///
    /// Panics if a layout has already been registered for the same path.
    #[must_use]
    pub fn layout(mut self, layout: impl ModuleLayout) -> Self {
        let path = self.module_path_to_path(layout.module_path());
        self.inner = self.inner.layout(ResolvedLayout::new(layout, path));
        self
    }

    /// Registers a [`ModuleRoute`], computing its route path from the module path.
    ///
    /// # Panics
    ///
    /// Panics if a route has already been registered for the same path.
    #[must_use]
    pub fn route(mut self, route: impl ModuleRoute) -> Self {
        let path = self.module_path_to_path(route.module_path());
        self.inner = self.inner.route(ResolvedRoute::new(route, path));
        self
    }

    /// Registers a [`ModuleLayer`], computing its path prefix from the module path.
    #[must_use]
    pub fn layer(mut self, layer: impl ModuleLayer) -> Self {
        let path = self.module_path_to_path(layer.module_path());
        self.inner = self.inner.layer(ResolvedLayer::new(layer, path));
        self
    }

    /// Registers every [`Segment`] override declared with `segment!` and
    /// collected at link time.
    ///
    /// Segments must be registered before any pages or layouts, since they
    /// affect path computation.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_segments(mut self) -> Self {
        for segment in inventory::iter::<Segment>().cloned() {
            self = self.segment(segment);
        }
        self
    }

    /// Registers every [`ModulePage`] annotated with `#[page]` and collected
    /// at link time, deriving each path from the module tree.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_pages(mut self) -> Self {
        for &page in inventory::iter::<&'static dyn ModulePage>() {
            self = self.page(page);
        }
        self
    }

    /// Registers every [`ModuleLayout`] annotated with `#[layout]` and
    /// collected at link time, deriving each path from the module tree.
    ///
    /// At most one discovered layout is allowed per path: a page's layouts nest
    /// by path prefix, so two layouts resolving to the same path would have an
    /// undefined nesting order. To attach more than one layout to a page, give
    /// them distinct paths or compose them in a single layout component.
    ///
    /// # Panics
    ///
    /// Panics if two discovered layouts resolve to the same path.
    #[cfg(feature = "discover")]
    #[must_use]
    #[track_caller]
    pub fn discover_layouts(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        for &layout in inventory::iter::<&'static dyn ModuleLayout>() {
            assert!(
                seen.insert(self.module_path_to_path(layout.module_path())),
                "multiple discovered layouts registered for the same path \"{}\"",
                self.module_path_to_path(layout.module_path())
            );
            self = self.layout(layout);
        }
        self
    }

    /// Registers every [`ModuleRoute`] annotated with `#[route]` and collected
    /// at link time, deriving each path from the module tree.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_routes(mut self) -> Self {
        for &route in inventory::iter::<&'static dyn ModuleRoute>() {
            self = self.route(route);
        }
        self
    }

    /// Registers every [`ModuleLayer`] annotated with `#[layer]` and collected
    /// at link time, deriving each path from the module tree.
    ///
    /// At most one discovered layer is allowed per path. Link-time collection
    /// order is non-deterministic, so two discovered layers sharing a path would
    /// have an undefined run order; this rejects that rather than pick an
    /// arbitrary one. To stack several layers on one path, register them
    /// manually with [`RouterBuilder::layer`](crate::RouterBuilder::layer),
    /// whose order is well-defined.
    ///
    /// # Panics
    ///
    /// Panics if two discovered layers resolve to the same path.
    #[cfg(feature = "discover")]
    #[must_use]
    #[track_caller]
    pub fn discover_layers(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        for &layer in inventory::iter::<&'static dyn ModuleLayer>() {
            assert!(
                seen.insert(self.module_path_to_path(layer.module_path())),
                "multiple discovered layers registered for the same path \"{}\"",
                self.module_path_to_path(layer.module_path())
            );
            self = self.layer(layer);
        }
        self
    }

    /// Discovers and registers all segments, pages, layouts, routes, and layers
    /// collected at link time.
    ///
    /// Segments are registered first, since they must precede pages and
    /// layouts.
    ///
    /// # Panics
    ///
    /// Panics if two discovered layers resolve to the same path; see
    /// [`discover_layers`](Self::discover_layers).
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover(self) -> Self {
        self.discover_segments()
            .discover_pages()
            .discover_layouts()
            .discover_routes()
            .discover_layers()
    }
}

impl From<ModuleRouterBuilder> for RouterBuilder {
    fn from(value: ModuleRouterBuilder) -> Self {
        value.inner
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;
    use topcoat_view::BoxView;

    use super::*;
    use crate::{Body, Method, Methods, RouteId};

    /// A [`ModulePage`] whose render function is never invoked; used to
    /// exercise registration and path computation without running a page.
    struct PageAt {
        id: RouteId,
        module_path: &'static str,
    }

    impl ModulePage for PageAt {
        fn id(&self) -> RouteId {
            self.id
        }

        fn methods(&self) -> Methods<'_> {
            Methods::Only(&[Method::GET])
        }

        fn module_path(&self) -> &'static str {
            self.module_path
        }

        fn render<'a>(&'a self, _cx: &'a Cx, _body: Body) -> BoxView<'a> {
            unreachable!("test render function is never called")
        }
    }

    fn page_at(module_path: &'static str) -> PageAt {
        PageAt {
            id: RouteId::new(),
            module_path,
        }
    }

    fn builder() -> ModuleRouterBuilder {
        ModuleRouterBuilder::new("app")
    }

    // -- relative_module_path --

    #[test]
    fn relative_path_of_root_is_empty() {
        assert_eq!(builder().relative_module_path("app"), "");
    }

    #[test]
    fn relative_path_strips_root_prefix() {
        assert_eq!(builder().relative_module_path("app::users"), "users");
        assert_eq!(
            builder().relative_module_path("app::users::id"),
            "users::id"
        );
    }

    #[test]
    #[should_panic(expected = "module path must be under module router's root")]
    fn relative_path_outside_root_panics() {
        builder().relative_module_path("other::thing");
    }

    #[test]
    #[should_panic(expected = "module path must be under module router's root")]
    fn relative_path_requires_module_boundary() {
        // `application` shares the `app` prefix but is not a submodule of it.
        builder().relative_module_path("application");
    }

    // -- module_path_to_path --

    fn path_of(module_path: &'static str) -> String {
        builder().module_path_to_path(module_path).to_string()
    }

    #[test]
    fn root_maps_to_empty_path() {
        assert_eq!(path_of("app"), "");
    }

    #[test]
    fn static_segment_is_kebab_cased() {
        assert_eq!(path_of("app::about"), "/about");
        assert_eq!(path_of("app::blog_posts"), "/blog-posts");
    }

    #[test]
    fn nested_static_segments() {
        assert_eq!(path_of("app::settings::profile"), "/settings/profile");
        assert_eq!(
            path_of("app::user_settings::email_address"),
            "/user-settings/email-address"
        );
    }

    #[test]
    fn underscore_module_is_a_group() {
        let path = builder().module_path_to_path("app::_marketing::pricing");
        // The group segment is recorded in the path but stripped from the URL.
        assert_eq!(path.to_string(), "/(_marketing)/pricing");
        assert_eq!(path.to_matchit_path(), "/pricing");
    }

    // -- module_path_to_path with segment overrides --

    fn builder_with(segment: Segment) -> ModuleRouterBuilder {
        builder().segment(segment)
    }

    #[test]
    fn override_kind_param() {
        let builder = builder_with(Segment::new(
            "app::users::id",
            Some(SegmentKind::Param),
            None,
        ));
        assert_eq!(
            builder.module_path_to_path("app::users::id").to_string(),
            "/users/{id}"
        );
    }

    #[test]
    fn override_kind_catch_all() {
        let builder = builder_with(Segment::new(
            "app::files::rest",
            Some(SegmentKind::CatchAll),
            None,
        ));
        assert_eq!(
            builder.module_path_to_path("app::files::rest").to_string(),
            "/files/{*rest}"
        );
    }

    #[test]
    fn override_kind_group_strips_from_url() {
        let builder = builder_with(Segment::new(
            "app::marketing",
            Some(SegmentKind::Group),
            None,
        ));
        let path = builder.module_path_to_path("app::marketing::pricing");
        assert_eq!(path.to_string(), "/(marketing)/pricing");
        assert_eq!(path.to_matchit_path(), "/pricing");
    }

    #[test]
    fn override_kind_static_promotes_group_module() {
        // A `_`-prefixed module forced back to a static URL segment.
        let builder = builder_with(Segment::new("app::_group", Some(SegmentKind::Static), None));
        assert_eq!(
            builder.module_path_to_path("app::_group").to_string(),
            "/group"
        );
    }

    #[test]
    fn override_rename_is_used_verbatim() {
        // A rename is used as-is, without kebab-casing.
        let builder = builder_with(Segment::new(
            "app::blog_post",
            None,
            Some("articles".into()),
        ));
        assert_eq!(
            builder.module_path_to_path("app::blog_post").to_string(),
            "/articles"
        );
    }

    #[test]
    fn override_applies_at_intermediate_segment() {
        let builder = builder_with(Segment::new("app::users", Some(SegmentKind::Param), None));
        assert_eq!(
            builder.module_path_to_path("app::users::posts").to_string(),
            "/{users}/posts"
        );
    }

    // -- segment registration --

    #[test]
    #[should_panic(expected = "must be called before registering any resource")]
    fn segment_after_resource_panics() {
        let _ = builder().page(page_at("app::home")).segment(Segment::new(
            "app::users",
            Some(SegmentKind::Param),
            None,
        ));
    }

    // -- resource registration and conversion --

    #[test]
    fn fresh_builder_converts_to_empty_router_builder() {
        let inner = RouterBuilder::from(builder());
        assert!(inner.is_empty());
    }

    #[test]
    fn registering_a_page_is_observable_after_conversion() {
        let inner = RouterBuilder::from(builder().page(page_at("app::about")));
        assert!(!inner.is_empty());
    }
}
