use std::borrow::Cow;

use heck::ToKebabCase;

use super::{
    layer::ResolvedLayer,
    page::{ResolvedLayout, ResolvedPage},
    route::ResolvedRoute,
};
use crate::{
    ModuleLayer, ModuleLayout, ModulePage, ModuleRoute, Path, PathBuf, PathSegment, RouterBuilder,
    Segment, SegmentKind, Segments,
};

/// A router builder that derives route paths from Rust module paths, created
/// by the `module_router!` macro.
///
/// The module tree under `root_module_path` becomes the route tree. Each
/// module becomes one path segment: `_`-prefixed modules become groups, and
/// other module names become kebab-cased static segments. A `segment!`
/// declaration in a module overrides its kind or name. Convert the builder
/// into a [`RouterBuilder`] with `From` when done.
///
/// A module path becomes a route path in these steps:
/// 1. Strip the `root_module_path` prefix
/// 2. Walk each `::`-separated component, checking for a [`Segment`] override
/// 3. Apply default kind (`_` prefix -> `Group`, otherwise `Static`)
/// 4. Kebab-case static segment names, leave others as-is
/// 5. Collect into a [`PathBuf`]
/// 6. Append the handler's relative path, if it declares one
#[doc(hidden)]
pub struct ModuleRouterBuilder {
    inner: RouterBuilder,
    root_module_path: &'static str,
    segments: Segments,
}

impl ModuleRouterBuilder {
    /// Creates a builder rooted at `root_module_path`, the `module_path!()` of
    /// the module that calls `module_router!`.
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
    /// Panics if a page, layout, or route is already registered, since
    /// overrides change how their paths are computed. Also panics if the
    /// module already has an override, or if it is not inside the root
    /// module.
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

    /// Computes the route path of a handler declared in `module_path` with
    /// `relative_path` below it. The root relative path stands for the module
    /// path with a trailing slash.
    ///
    /// # Panics
    ///
    /// Panics if the trailing slash is asked of the root path, which has none.
    fn resolve_path(&self, module_path: &'static str, relative_path: Option<&Path>) -> PathBuf {
        let mut path = self.module_path_to_path(module_path);
        match relative_path {
            None => {}
            Some(relative_path) if relative_path.is_empty() => {
                assert!(
                    !path.is_empty(),
                    "the root path has no trailing slash form; `./` cannot be used in the \
                     module router's root module"
                );
                path += PathSegment::Static("");
            }
            Some(relative_path) => path += relative_path,
        }
        path
    }

    /// Registers a [`ModulePage`] at the path derived from its module path.
    ///
    /// # Panics
    ///
    /// Panics if the page's module is not inside the root module, or if it
    /// uses a `./` path in the root module.
    #[must_use]
    pub fn page(mut self, page: impl ModulePage) -> Self {
        let path = self.resolve_path(page.module_path(), page.relative_path());
        self.inner = self.inner.page(ResolvedPage::new(page, path));
        self
    }

    /// Registers a [`ModuleLayout`] at the path derived from its module path.
    ///
    /// # Panics
    ///
    /// Panics if the layout's module is not inside the root module, or if it
    /// uses a `./` path in the root module.
    #[must_use]
    pub fn layout(mut self, layout: impl ModuleLayout) -> Self {
        let path = self.resolve_path(layout.module_path(), layout.relative_path());
        self.inner = self.inner.layout(ResolvedLayout::new(layout, path));
        self
    }

    /// Registers a [`ModuleRoute`] at the path derived from its module path.
    ///
    /// # Panics
    ///
    /// Panics if the route's module is not inside the root module, or if it
    /// uses a `./` path in the root module.
    #[must_use]
    pub fn route(mut self, route: impl ModuleRoute) -> Self {
        let path = self.resolve_path(route.module_path(), route.relative_path());
        self.inner = self.inner.route(ResolvedRoute::new(route, path));
        self
    }

    /// Registers a [`ModuleLayer`] at the path derived from its module path.
    ///
    /// # Panics
    ///
    /// Panics if the layer's module is not inside the root module, or if it
    /// uses a `./` path in the root module.
    #[must_use]
    pub fn layer(mut self, layer: impl ModuleLayer) -> Self {
        let path = self.resolve_path(layer.module_path(), layer.relative_path());
        self.inner = self.inner.layer(ResolvedLayer::new(layer, path));
        self
    }

    /// Registers every [`Segment`] override declared with `segment!` in the
    /// program.
    ///
    /// Call this before registering any page, layout, or route, since
    /// overrides change how their paths are computed.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_segments(mut self) -> Self {
        for segment in inventory::iter::<Segment>().cloned() {
            self = self.segment(segment);
        }
        self
    }

    /// Registers every [`ModulePage`] declared with `#[page]` in the program,
    /// at the path derived from its module path.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_pages(mut self) -> Self {
        for &page in inventory::iter::<&'static dyn ModulePage>() {
            self = self.page(page);
        }
        self
    }

    /// Registers every [`ModuleLayout`] declared with `#[layout]` in the
    /// program, at the path derived from its module path.
    ///
    /// Only one discovered layout is allowed per path. Layouts nest by path,
    /// so two layouts at the same path would have no defined order. To wrap a
    /// page in more than one layout, give the layouts different paths or
    /// combine them into one layout component.
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
            let path = self.resolve_path(layout.module_path(), layout.relative_path());
            assert!(
                seen.insert(path.clone()),
                "multiple discovered layouts registered for the same path \"{path}\"",
            );
            self = self.layout(layout);
        }
        self
    }

    /// Registers every [`ModuleRoute`] declared with `#[route]` in the
    /// program, at the path derived from its module path.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_routes(mut self) -> Self {
        for &route in inventory::iter::<&'static dyn ModuleRoute>() {
            self = self.route(route);
        }
        self
    }

    /// Registers every [`ModuleLayer`] declared with `#[layer]` in the
    /// program, at the path derived from its module path.
    ///
    /// Only one discovered layer is allowed per path. Discovered layers have
    /// no defined order, so two layers at the same path would run in an
    /// unpredictable order. To stack several layers on one path, register
    /// them with [`RouterBuilder::layer`](crate::RouterBuilder::layer)
    /// instead.
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
            let path = self.resolve_path(layer.module_path(), layer.relative_path());
            assert!(
                seen.insert(path.clone()),
                "multiple discovered layers registered for the same path \"{path}\"",
            );
            self = self.layer(layer);
        }
        self
    }

    /// Registers every segment override, page, layout, route, and layer
    /// declared in the program.
    ///
    /// Segment overrides are registered first, since they change how the
    /// other paths are computed.
    ///
    /// # Panics
    ///
    /// Panics if two discovered layouts or two discovered layers resolve to
    /// the same path, or for any reason the individual `discover_*` methods
    /// list.
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
        relative_path: Option<&'static Path>,
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

        fn relative_path(&self) -> Option<&Path> {
            self.relative_path
        }

        fn render<'a>(&'a self, _cx: &'a Cx, _body: Body) -> BoxView<'a> {
            unreachable!("test render function is never called")
        }
    }

    fn page_at(module_path: &'static str) -> PageAt {
        PageAt {
            id: RouteId::new(),
            module_path,
            relative_path: None,
        }
    }

    fn page_below(module_path: &'static str, relative_path: &'static Path) -> PageAt {
        PageAt {
            id: RouteId::new(),
            module_path,
            relative_path: Some(relative_path),
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

    // -- resolve_path --

    fn resolved_path(page: &PageAt) -> String {
        builder()
            .resolve_path(page.module_path(), page.relative_path())
            .to_string()
    }

    #[test]
    fn resolve_path_without_relative_path_is_module_path() {
        assert_eq!(resolved_path(&page_at("app::settings")), "/settings");
    }

    #[test]
    fn resolve_path_appends_relative_path() {
        assert_eq!(
            resolved_path(&page_below("app::settings", Path::new("/export"))),
            "/settings/export"
        );
        assert_eq!(
            resolved_path(&page_below("app::users", Path::new("/{id}/posts"))),
            "/users/{id}/posts"
        );
    }

    #[test]
    fn resolve_path_at_root_module() {
        assert_eq!(
            resolved_path(&page_below("app", Path::new("/about"))),
            "/about"
        );
    }

    #[test]
    fn resolve_path_root_relative_path_adds_a_trailing_slash() {
        assert_eq!(
            resolved_path(&page_below("app::settings", Path::ROOT)),
            "/settings/"
        );
        assert_eq!(
            resolved_path(&page_below("app::users::posts", Path::ROOT)),
            "/users/posts/"
        );
    }

    #[test]
    fn resolve_path_keeps_a_relative_trailing_slash() {
        assert_eq!(
            resolved_path(&page_below("app::settings", Path::new("/export/"))),
            "/settings/export/"
        );
    }

    #[test]
    #[should_panic(expected = "the root path has no trailing slash form")]
    fn resolve_path_rejects_a_trailing_slash_at_the_root_module() {
        resolved_path(&page_below("app", Path::ROOT));
    }

    #[test]
    fn resolve_path_applies_segment_overrides_before_relative_path() {
        let builder = builder_with(Segment::new("app::users", Some(SegmentKind::Param), None));
        assert_eq!(
            builder
                .resolve_path("app::users", Some(Path::new("/edit")))
                .to_string(),
            "/{users}/edit"
        );
    }

    #[test]
    fn pages_in_one_module_with_distinct_relative_paths_both_register() {
        // Two pages at the same path would panic, so distinct relative paths
        // must keep them apart.
        let inner = RouterBuilder::from(
            builder()
                .page(page_at("app::settings"))
                .page(page_below("app::settings", Path::new("/export"))),
        );
        assert!(!inner.is_empty());
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
