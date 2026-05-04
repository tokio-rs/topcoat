use std::borrow::Cow;

use heck::ToKebabCase;

use crate::{
    ModuleLayout, ModulePage, PathBuf, PathSegment, Router, Segment, SegmentKind, Segments,
};

/// The module-based router, created by the `module_router!` macro.
///
/// Translates Rust module paths into route paths and builds a [`Router`].
/// The module tree rooted at `root_module_path` becomes the route tree: each
/// module maps to a path segment, with `_`-prefixed modules becoming groups
/// and static names kebab-cased.
///
/// Segment overrides (kind, rename) registered via `segment!` are applied during
/// the module-to-path translation. Overrides must be registered before any pages
/// or layouts — this is enforced with a panic.
///
/// The translation pipeline for a given module path:
/// 1. Strip the `root_module_path` prefix
/// 2. Walk each `::`-separated component, checking for a [`Segment`] override
/// 3. Apply default kind (`_` prefix → `Group`, otherwise `Static`)
/// 4. Kebab-case static segment names, leave others as-is
/// 5. Collect into a [`PathBuf`]
#[doc(hidden)]
pub struct ModuleRouter {
    inner: Router,
    root_module_path: &'static str,
    segments: Segments,
}

impl ModuleRouter {
    /// Creates a new `ModuleRouter` rooted at the given module path.
    ///
    /// The `root_module_path` is the `module_path!()` of the module calling
    /// `module_router!`.
    pub fn new(root_module_path: &'static str) -> Self {
        Self {
            root_module_path,
            inner: Router::new(),
            segments: Segments::new(),
        }
    }

    /// Returns the module path relative to the root.
    ///
    /// This is the key used to look up [`Segment`] overrides and to derive
    /// route path segments.
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
                None => match component.starts_with("_") {
                    true => SegmentKind::Group,
                    false => SegmentKind::Static,
                },
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
    pub fn page(mut self, page: ModulePage) -> Self {
        let module_path = page.module_path();
        let page = page.into_page(Cow::Owned(self.module_path_to_path(module_path)));
        self.inner = self.inner.page(page);
        self
    }

    /// Registers a [`ModuleLayout`], computing its route path from the module path.
    ///
    /// # Panics
    ///
    /// Panics if a layout has already been registered for the same path.
    pub fn layout(mut self, layout: ModuleLayout) -> Self {
        let module_path = layout.module_path();
        let layout = layout.into_layout(Cow::Owned(self.module_path_to_path(module_path)));
        self.inner = self.inner.layout(layout);
        self
    }

    /// Discovers and registers all segments, pages, and layouts collected at link time.
    ///
    /// Segments are registered first (they must precede pages/layouts), then
    /// pages and layouts, and finally the inner router's own `discover()` is
    /// called to pick up any non-module-router pages and layouts.
    #[cfg(feature = "discover")]
    pub fn discover(mut self) -> Self {
        for segment in inventory::iter::<Segment>().cloned() {
            self = self.segment(segment);
        }
        for page in inventory::iter::<ModulePage>().cloned() {
            self = self.page(page);
        }
        for layout in inventory::iter::<ModuleLayout>().cloned() {
            self = self.layout(layout);
        }
        self.inner = self.inner.discover();
        self
    }
}

impl From<ModuleRouter> for Router {
    fn from(value: ModuleRouter) -> Self {
        value.inner
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use topcoat_view::runtime::View;

    use super::*;

    fn router(root: &'static str) -> ModuleRouter {
        ModuleRouter::new(root)
    }

    fn seg(
        module_path: &'static str,
        kind: Option<SegmentKind>,
        rename: Option<&'static str>,
    ) -> Segment {
        Segment::new(module_path, kind, rename.map(Cow::Borrowed))
    }

    // ── module_path_to_path: basic static routes ─────────────────────

    #[test]
    fn root_page() {
        let r = router("my_crate::app");
        assert_eq!(r.module_path_to_path("my_crate::app").to_string(), "");
    }

    #[test]
    fn simple_page() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::about").to_string(),
            "/about"
        );
    }

    #[test]
    fn nested_page() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::settings::profile")
                .to_string(),
            "/settings/profile"
        );
    }

    #[test]
    fn nested_mod() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::settings").to_string(),
            "/settings"
        );
    }

    // ── module_path_to_path: kebab-case conversion ───────────────────

    #[test]
    fn static_segment_is_kebab_cased() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::my_page").to_string(),
            "/my-page"
        );
    }

    #[test]
    fn nested_static_segments_are_kebab_cased() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::user_settings::change_password")
                .to_string(),
            "/user-settings/change-password"
        );
    }

    // ── module_path_to_path: group segments (underscore prefix) ──────

    #[test]
    fn group_segment() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::_group::contact")
                .to_string(),
            "/(_group)/contact"
        );
    }

    #[test]
    fn group_mod() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::_group").to_string(),
            "/(_group)"
        );
    }

    #[test]
    fn nested_groups() {
        let r = router("my_crate::app");
        assert_eq!(
            r.module_path_to_path("my_crate::app::_auth::_admin::dashboard")
                .to_string(),
            "/(_auth)/(_admin)/dashboard"
        );
    }

    // ── module_path_to_path: segment overrides ───────────────────────

    #[test]
    fn override_static_to_param() {
        let r = router("my_crate::app").segment(seg(
            "my_crate::app::user_id",
            Some(SegmentKind::Param),
            None,
        ));
        assert_eq!(
            r.module_path_to_path("my_crate::app::user_id").to_string(),
            "/{user_id}"
        );
    }

    #[test]
    fn override_static_to_catch_all() {
        let r = router("my_crate::app").segment(seg(
            "my_crate::app::rest",
            Some(SegmentKind::CatchAll),
            None,
        ));
        assert_eq!(
            r.module_path_to_path("my_crate::app::rest").to_string(),
            "/{*rest}"
        );
    }

    #[test]
    fn override_group_to_static() {
        let r = router("my_crate::app").segment(seg(
            "my_crate::app::_internal",
            Some(SegmentKind::Static),
            None,
        ));
        // Overridden to static, so kebab-case is applied to the "_internal" name.
        assert_eq!(
            r.module_path_to_path("my_crate::app::_internal::page")
                .to_string(),
            "/internal/page"
        );
    }

    #[test]
    fn rename_segment() {
        let r =
            router("my_crate::app").segment(seg("my_crate::app::blog_post", None, Some("posts")));
        assert_eq!(
            r.module_path_to_path("my_crate::app::blog_post")
                .to_string(),
            "/posts"
        );
    }

    #[test]
    fn rename_and_kind_override() {
        let r = router("my_crate::app").segment(seg(
            "my_crate::app::slug",
            Some(SegmentKind::Param),
            Some("id"),
        ));
        assert_eq!(
            r.module_path_to_path("my_crate::app::slug").to_string(),
            "/{id}"
        );
    }

    #[test]
    fn param_in_nested_path() {
        let r = router("my_crate::app").segment(seg(
            "my_crate::app::users::id",
            Some(SegmentKind::Param),
            None,
        ));
        assert_eq!(
            r.module_path_to_path("my_crate::app::users::id")
                .to_string(),
            "/users/{id}"
        );
        assert_eq!(
            r.module_path_to_path("my_crate::app::users::id::settings")
                .to_string(),
            "/users/{id}/settings"
        );
    }

    #[test]
    fn catch_all_nested() {
        let r = router("my_crate::app").segment(seg(
            "my_crate::app::docs::path",
            Some(SegmentKind::CatchAll),
            None,
        ));
        assert_eq!(
            r.module_path_to_path("my_crate::app::docs::path")
                .to_string(),
            "/docs/{*path}"
        );
    }

    // ── module_path_to_path: multiple segment overrides ──────────────

    #[test]
    fn multiple_segments() {
        let r = router("my_crate::app")
            .segment(seg(
                "my_crate::app::users::id",
                Some(SegmentKind::Param),
                None,
            ))
            .segment(seg(
                "my_crate::app::users::id::posts::post_id",
                Some(SegmentKind::Param),
                None,
            ));
        assert_eq!(
            r.module_path_to_path("my_crate::app::users::id::posts::post_id")
                .to_string(),
            "/users/{id}/posts/{post_id}"
        );
    }

    // ── segment ordering assertion ───────────────────────────────────

    #[test]
    #[should_panic(expected = "`segment` must be called before registering any resource")]
    fn segment_after_page_panics() {
        let r = router("my_crate::app");
        // Register a page first, then try to add a segment.
        let page = ModulePage::new("my_crate::app::about", || {
            Box::pin(async { Ok(View::new("")) })
        });
        r.page(page)
            .segment(seg("my_crate::app::users", Some(SegmentKind::Param), None));
    }
}
