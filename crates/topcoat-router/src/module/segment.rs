use std::{borrow::Cow, collections::HashMap};

/// The kind of path segment a module adds under `module_router!`.
///
/// By default, a module adds a `Static` segment, and a module whose name
/// starts with `_` adds a `Group`. Call the `segment!` macro in a module to
/// choose another kind.
///
/// # Examples
///
/// ```rust
/// // In src/app/users/id.rs: pages in this module serve `/users/{id}`.
/// topcoat::router::segment!(kind = Param);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SegmentKind {
    /// A literal URL segment, like `/users`. The default for regular modules.
    Static,
    /// A group that adds no URL segment but can hold layouts and layers. The
    /// default for `_`-prefixed modules.
    Group,
    /// A path parameter that matches one segment, like `/{id}`.
    Param,
    /// A catch-all that matches all remaining segments, like `/{*path}`.
    CatchAll,
}

/// A segment override for one module, created by the `segment!` macro.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub struct Segment {
    /// The module path, set by the `segment!` macro with `module_path!()`.
    module_path: &'static str,
    /// Overridden segment kind, or `None` to use the default (static / group).
    kind: Option<SegmentKind>,
    /// Overridden URL name, or `None` to derive from the module name.
    rename: Option<Cow<'static, str>>,
}

impl Segment {
    /// Creates a segment override. The `segment!` macro calls this.
    #[must_use]
    pub const fn new(
        module_path: &'static str,
        kind: Option<SegmentKind>,
        rename: Option<Cow<'static, str>>,
    ) -> Self {
        Self {
            module_path,
            kind,
            rename,
        }
    }

    /// Returns the module path that declared this segment.
    #[must_use]
    pub fn module_path(&self) -> &'static str {
        self.module_path
    }

    /// Returns the overridden [`SegmentKind`], if any.
    #[must_use]
    pub fn kind(&self) -> Option<&SegmentKind> {
        self.kind.as_ref()
    }

    /// Returns the overridden URL name, if any.
    #[must_use]
    pub fn rename(&self) -> Option<&str> {
        self.rename.as_deref()
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Segment);

/// The [`Segment`] overrides registered on a module router, keyed by module
/// path.
#[doc(hidden)]
#[derive(Debug, Default, Clone)]
pub(crate) struct Segments {
    segments: HashMap<&'static str, Segment>,
}

impl Segments {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Segments::default()
    }

    /// Registers the override for the module at `path`.
    ///
    /// # Panics
    ///
    /// Panics if the module already has an override.
    #[track_caller]
    pub fn register(&mut self, path: &'static str, segment: Segment) {
        if let Some(existing) = self.segments.insert(path, segment) {
            panic!(
                "duplicate segment specifier in `{}`",
                existing.module_path()
            )
        }
    }

    /// Returns the override for the module at `path`, if any.
    pub fn get(&self, path: &str) -> Option<&Segment> {
        self.segments.get(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_segment() -> Segment {
        Segment::new("my_crate::test", Some(SegmentKind::Static), None)
    }

    #[test]
    fn register_and_get() {
        let mut segments = Segments::new();
        segments.register("foo", test_segment());

        let seg = segments.get("foo").unwrap();
        assert_eq!(seg.module_path(), "my_crate::test");
        assert_eq!(seg.kind(), Some(&SegmentKind::Static));
        assert_eq!(seg.rename(), None);
    }

    #[test]
    fn get_missing_returns_none() {
        let segments = Segments::new();
        assert!(segments.get("nope").is_none());
    }

    #[test]
    #[should_panic(expected = "duplicate segment specifier")]
    fn register_duplicate_panics() {
        let mut segments = Segments::new();
        segments.register("foo", test_segment());
        segments.register("foo", test_segment());
    }
}
