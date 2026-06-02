use std::{
    borrow::{Borrow, Cow},
    fmt::{Display, Write},
    ops::{AddAssign, Deref},
};

use ref_cast::{RefCastCustom, ref_cast_custom};

/// A borrowed route path, similar to [`std::path::Path`] but for URL paths.
///
/// A `Path` consists of `/`-separated segments, where each segment is one of:
/// - **Static** — a literal string (e.g. `users`)
/// - **Param** — a dynamic parameter in braces (e.g. `{id}`)
/// - **CatchAll** — a wildcard tail in braces with `*` (e.g. `{*rest}`)
/// - **Group** — a logical grouping in parentheses (e.g. `(auth)`), stripped when converting to an Axum path
///
/// The root path `"/"` is normalized to an empty inner string. Use [`Path::new`] to
/// create a `&Path` from a string slice.
///
/// # Examples
///
/// ```
/// use topcoat_router::Path;
///
/// let path = Path::new("/users/(group)/{id}");
/// assert_eq!(path.segments().count(), 3);
/// assert_eq!(path.to_axum_path(), "/users/{id}");
/// ```
#[derive(Debug, PartialEq, Eq, Hash, RefCastCustom)]
#[repr(transparent)]
pub struct Path {
    inner: str,
}

impl Path {
    /// Creates a `&Path` from a string slice.
    ///
    /// The root path `"/"` is normalized to an empty inner representation so that
    /// it produces zero segments, matching the convention that the root layout
    /// applies to all pages.
    pub const fn new(s: &str) -> &Self {
        let s = match s.as_bytes() {
            [b'/'] => "",
            _ => s,
        };
        Self::from_str(s)
    }

    #[ref_cast_custom]
    const fn from_str(s: &str) -> &Self;

    /// Returns an iterator over the [`PathSegment`]s of this path.
    ///
    /// The root path yields zero segments.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::{Path, PathSegment};
    ///
    /// let path = Path::new("/users/{id}/(auth)");
    /// let segs: Vec<_> = path.segments().collect();
    /// assert_eq!(segs, vec![
    ///     PathSegment::Static("users"),
    ///     PathSegment::Param("id"),
    ///     PathSegment::Group("auth"),
    /// ]);
    /// ```
    pub fn segments(&self) -> impl Iterator<Item = PathSegment<'_>> {
        self.inner.split("/").skip(1).map(PathSegment::new)
    }

    /// Converts this path to an Axum-compatible path string, stripping group segments.
    ///
    /// Group segments (e.g. `(auth)`) are used for layout matching but are not
    /// part of the URL that Axum routes against. This method removes them and
    /// returns the remaining path.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::Path;
    ///
    /// let path = Path::new("/(auth)/dashboard/{id}");
    /// assert_eq!(path.to_axum_path(), "/dashboard/{id}");
    ///
    /// let root = Path::new("/");
    /// assert_eq!(root.to_axum_path(), "/");
    ///
    /// // A path made up entirely of group segments collapses to the root URL,
    /// // e.g. a page in a `(marketing)` group that should serve `/`.
    /// let group_root = Path::new("/(marketing)");
    /// assert_eq!(group_root.to_axum_path(), "/");
    /// ```
    pub fn to_axum_path(&self) -> Cow<'static, str> {
        if self.inner.is_empty() {
            return Cow::Borrowed("/");
        }
        let stripped = self
            .segments()
            .filter(|s| !s.is_group())
            .collect::<PathBuf>()
            .inner;
        // Stripping groups can leave nothing behind (e.g. `/(marketing)` or
        // `/(a)/(b)`). Such a path addresses the root URL, so normalize the empty
        // result back to "/" — Axum rejects route paths that don't start with "/".
        if stripped.is_empty() {
            return Cow::Borrowed("/");
        }
        Cow::Owned(stripped)
    }

    /// Returns `true` if this path starts with the given prefix path.
    ///
    /// Comparison is done segment-by-segment using [`PathSegment`] equality.
    /// This is used to determine which layouts apply to a given page — a layout
    /// at `"/settings"` matches any page whose path starts with `/settings`.
    ///
    /// Group segments are included in the comparison.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::Path;
    ///
    /// let path = Path::new("/users/{id}/posts");
    /// assert!(path.starts_with(Path::new("/users/{id}")));
    /// assert!(!path.starts_with(Path::new("/posts/{id}")));
    /// ```
    pub fn starts_with(&self, other: &Path) -> bool {
        if self.inner.len() < other.inner.len() {
            return false;
        }
        return self.segments().zip(other.segments()).all(|(a, b)| a == b);
    }

    /// Returns the string backing this path.
    ///
    /// This length is in bytes, not [`char`]s or graphemes. In other words,
    /// it might not be what a human considers the length of the string.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if `self` has no path segments, i.e. `self` is the root path `/`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl ToOwned for Path {
    type Owned = PathBuf;

    fn to_owned(&self) -> Self::Owned {
        PathBuf {
            inner: self.inner.to_owned(),
        }
    }
}

/// An owned route path, similar to [`std::path::PathBuf`] but for URL paths.
///
/// `PathBuf` is the owned counterpart of [`Path`]. It can be built incrementally
/// by adding [`PathSegment`]s with `+=`, or collected from an iterator of segments.
///
/// # Examples
///
/// ```
/// use topcoat_router::{PathBuf, PathSegment};
///
/// let mut buf = PathBuf::new();
/// buf += PathSegment::Static("users");
/// buf += PathSegment::Param("id");
/// assert_eq!(buf.to_string(), "/users/{id}");
/// ```
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct PathBuf {
    inner: String,
}

impl PathBuf {
    /// Creates a new empty `PathBuf`.
    pub fn new() -> Self {
        Default::default()
    }
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        Path::new(&self.inner)
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        Path::new(&self.inner)
    }
}

impl AddAssign<PathSegment<'_>> for PathBuf {
    fn add_assign(&mut self, rhs: PathSegment<'_>) {
        write!(self.inner, "/{rhs}").unwrap();
    }
}

impl<'a> FromIterator<PathSegment<'a>> for PathBuf {
    fn from_iter<T: IntoIterator<Item = PathSegment<'a>>>(iter: T) -> Self {
        let mut buf = PathBuf::new();
        for segment in iter {
            buf += segment;
        }
        buf
    }
}

impl Display for PathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

/// A single segment of a route [`Path`].
///
/// Topcoat paths use four segment types:
///
/// | Syntax      | Variant    | Example     | Description                                             |
/// |-------------|------------|-------------|---------------------------------------------------------|
/// | `foo`       | `Static`   | `users`     | Literal URL segment                                     |
/// | `{name}`    | `Param`    | `{id}`      | Dynamic parameter, extracted at request time            |
/// | `{*name}`   | `CatchAll` | `{*path}`   | Wildcard tail, matches the rest of the URL              |
/// | `(name)`    | `Group`    | `(auth)`    | Logical grouping for layout matching, stripped from URL |
///
/// Segment names (for `Param`, `CatchAll`, and `Group`) must be valid
/// identifiers: starting with a letter or underscore, containing only
/// ASCII alphanumerics and underscores.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment<'a> {
    /// A literal URL segment (e.g. `users`).
    Static(&'a str),
    /// A logical grouping segment (e.g. `(auth)`), stripped from the URL path.
    Group(&'a str),
    /// A dynamic parameter segment (e.g. `{id}`).
    Param(&'a str),
    /// A wildcard tail segment (e.g. `{*rest}`), matching the remainder of the URL.
    CatchAll(&'a str),
}

impl<'a> PathSegment<'a> {
    /// Parses a single path segment string into a [`PathSegment`].
    ///
    /// # Panics
    ///
    /// Panics if the segment is malformed:
    /// - Empty string
    /// - Missing closing `}` or `)`
    /// - Invalid identifier (empty name, starts with a digit, contains non-alphanumeric/underscore characters)
    /// - Unexpected brackets in a static segment (e.g. `foo{bar}`)
    pub fn new(s: &'a str) -> Self {
        if s.starts_with('{') {
            if !s.ends_with('}') {
                panic!("invalid segment: missing closing `}}` in `{s}`");
            }
            let inner = &s[1..s.len() - 1];
            if let Some(name) = inner.strip_prefix('*') {
                assert_valid_ident(name, "catch-all", s);
                PathSegment::CatchAll(name)
            } else {
                assert_valid_ident(inner, "param", s);
                PathSegment::Param(inner)
            }
        } else if s.starts_with('(') {
            if !s.ends_with(')') {
                panic!("invalid segment: missing closing `)` in `{s}`");
            }
            let inner = &s[1..s.len() - 1];
            assert_valid_ident(inner, "group", s);
            PathSegment::Group(inner)
        } else {
            if s.is_empty() {
                panic!("invalid segment: empty string");
            }
            if s.contains('{') || s.contains('}') || s.contains('(') || s.contains(')') {
                panic!("invalid segment: unexpected brackets in `{s}`");
            }
            PathSegment::Static(s)
        }
    }

    /// Returns `true` if the segment is [`Static`].
    ///
    /// [`Static`]: PathSegment::Static
    #[must_use]
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static(..))
    }

    /// Returns `true` if the segment is [`Group`].
    ///
    /// [`Group`]: PathSegment::Group
    #[must_use]
    pub fn is_group(&self) -> bool {
        matches!(self, Self::Group(..))
    }

    /// Returns `true` if the segment is [`Param`].
    ///
    /// [`Param`]: PathSegment::Param
    #[must_use]
    pub fn is_param(&self) -> bool {
        matches!(self, Self::Param(..))
    }

    /// Returns `true` if the segment is [`CatchAll`].
    ///
    /// [`CatchAll`]: PathSegment::CatchAll
    #[must_use]
    pub fn is_catch_all(&self) -> bool {
        matches!(self, Self::CatchAll(..))
    }

    /// Returns the inner string if this is a [`Static`](PathSegment::Static) segment.
    pub fn as_static(&self) -> Option<&&'a str> {
        if let Self::Static(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the inner string if this is a [`Group`](PathSegment::Group) segment.
    pub fn as_group(&self) -> Option<&&'a str> {
        if let Self::Group(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the inner string if this is a [`Param`](PathSegment::Param) segment.
    pub fn as_param(&self) -> Option<&&'a str> {
        if let Self::Param(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the inner string if this is a [`CatchAll`](PathSegment::CatchAll) segment.
    pub fn as_catch_all(&self) -> Option<&&'a str> {
        if let Self::CatchAll(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

fn assert_valid_ident(name: &str, kind: &str, raw: &str) {
    if name.is_empty() {
        panic!("invalid segment: {kind} name must not be empty in `{raw}`");
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        panic!(
            "invalid segment: {kind} name `{name}` must start with a letter or underscore in `{raw}`"
        );
    }
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            panic!(
                "invalid segment: {kind} name `{name}` contains invalid character `{ch}` in `{raw}`"
            );
        }
    }
}

impl Display for PathSegment<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static(inner) => f.write_str(inner),
            Self::Param(inner) => write!(f, "{{{inner}}}"),
            Self::Group(inner) => write!(f, "({inner})"),
            Self::CatchAll(inner) => write!(f, "{{*{inner}}}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Path ──

    #[test]
    fn path_root_slash_normalized() {
        let path = Path::new("/");
        assert_eq!(&path.inner, "");
        assert_eq!(path.to_axum_path(), "/");
        assert_eq!(path.segments().count(), 0);
    }

    #[test]
    fn path_segments() {
        let path = Path::new("/dashboard/{id}/(auth)");
        let segs: Vec<_> = path.segments().collect();
        assert_eq!(
            segs,
            vec![
                PathSegment::Static("dashboard"),
                PathSegment::Param("id"),
                PathSegment::Group("auth"),
            ]
        );
    }

    #[test]
    fn path_single_segment() {
        let path = Path::new("/home");
        let segs: Vec<_> = path.segments().collect();
        assert_eq!(segs, vec![PathSegment::Static("home")]);
    }

    #[test]
    fn path_to_axum_strips_groups() {
        let path = Path::new("/(auth)/dashboard/{id}");
        assert_eq!(path.to_axum_path(), "/dashboard/{id}");
    }

    #[test]
    fn path_to_axum_empty() {
        let path = Path::new("");
        assert_eq!(path.to_axum_path(), "/");
    }

    #[test]
    fn path_to_axum_group_only_is_root() {
        // A page inside a route group that should serve `/`.
        assert_eq!(Path::new("/(marketing)").to_axum_path(), "/");
        // Nested groups collapse the same way.
        assert_eq!(Path::new("/(a)/(b)").to_axum_path(), "/");
    }

    #[test]
    fn path_to_axum_no_groups() {
        let path = Path::new("/users/{id}");
        assert_eq!(path.to_axum_path(), "/users/{id}");
    }

    #[test]
    fn path_starts_with_match() {
        let path = Path::new("/users/{id}/posts");
        let prefix = Path::new("/users/{id}");
        assert!(path.starts_with(prefix));
    }

    #[test]
    fn path_starts_with_no_match() {
        let path = Path::new("/users/{id}");
        let prefix = Path::new("/posts/{id}");
        assert!(!path.starts_with(prefix));
    }

    #[test]
    fn path_starts_with_longer_prefix() {
        let path = Path::new("/users");
        let prefix = Path::new("/users/{id}/posts");
        assert!(!path.starts_with(prefix));
    }

    #[test]
    fn path_display() {
        let path = Path::new("/users/{id}");
        assert_eq!(path.to_string(), "/users/{id}");
    }

    // ── PathBuf ──

    #[test]
    fn pathbuf_new_is_empty() {
        let buf = PathBuf::new();
        assert_eq!(buf.to_string(), "");
    }

    #[test]
    fn pathbuf_add_assign() {
        let mut buf = PathBuf::new();
        buf += PathSegment::Static("users");
        buf += PathSegment::Param("id");
        assert_eq!(buf.to_string(), "/users/{id}");
    }

    #[test]
    fn pathbuf_from_iterator() {
        let buf: PathBuf = vec![
            PathSegment::Static("api"),
            PathSegment::Static("v1"),
            PathSegment::Param("resource"),
        ]
        .into_iter()
        .collect();
        assert_eq!(buf.to_string(), "/api/v1/{resource}");
    }

    #[test]
    fn pathbuf_deref_to_path() {
        let mut buf = PathBuf::new();
        buf += PathSegment::Static("users");
        let path: &Path = &buf;
        let segs: Vec<_> = path.segments().collect();
        assert_eq!(segs, vec![PathSegment::Static("users")]);
    }

    #[test]
    fn pathbuf_to_owned_roundtrip() {
        let path = Path::new("/users/{id}");
        let buf = path.to_owned();
        assert_eq!(&*buf, path);
    }

    // ── PathSegment ──

    #[test]
    fn static_segment() {
        let seg = PathSegment::new("dashboard");
        assert!(seg.is_static());
        assert_eq!(seg.as_static(), Some(&"dashboard"));
    }

    #[test]
    fn param_segment() {
        let seg = PathSegment::new("{id}");
        assert!(seg.is_param());
        assert_eq!(seg.as_param(), Some(&"id"));
    }

    #[test]
    fn param_with_underscore() {
        let seg = PathSegment::new("{user_id}");
        assert!(seg.is_param());
        assert_eq!(seg.as_param(), Some(&"user_id"));
    }

    #[test]
    fn catch_all_segment() {
        let seg = PathSegment::new("{*rest}");
        assert!(matches!(seg, PathSegment::CatchAll("rest")));
    }

    #[test]
    fn group_segment() {
        let seg = PathSegment::new("(auth)");
        assert!(seg.is_group());
        assert_eq!(seg.as_group(), Some(&"auth"));
    }

    #[test]
    fn display_roundtrip() {
        for input in ["dashboard", "{id}", "{*rest}", "(auth)"] {
            assert_eq!(PathSegment::new(input).to_string(), input);
        }
    }

    #[test]
    #[should_panic(expected = "missing closing `}`")]
    fn param_missing_close() {
        PathSegment::new("{id");
    }

    #[test]
    #[should_panic(expected = "missing closing `)`")]
    fn group_missing_close() {
        PathSegment::new("(auth");
    }

    #[test]
    #[should_panic(expected = "invalid segment: empty string")]
    fn empty_segment() {
        PathSegment::new("");
    }

    #[test]
    #[should_panic(expected = "unexpected brackets")]
    fn static_with_braces() {
        PathSegment::new("foo{bar}");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn param_empty_name() {
        PathSegment::new("{}");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn group_empty_name() {
        PathSegment::new("()");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn catch_all_empty_name() {
        PathSegment::new("{*}");
    }

    #[test]
    #[should_panic(expected = "must start with a letter or underscore")]
    fn param_invalid_start() {
        PathSegment::new("{0id}");
    }

    #[test]
    #[should_panic(expected = "contains invalid character")]
    fn param_invalid_char() {
        PathSegment::new("{id-name}");
    }

    #[test]
    #[should_panic(expected = "must start with a letter or underscore")]
    fn group_invalid_start() {
        PathSegment::new("(0auth)");
    }

    #[test]
    #[should_panic(expected = "contains invalid character")]
    fn group_invalid_char() {
        PathSegment::new("(my-group)");
    }

    #[test]
    fn underscore_leading_ident() {
        let seg = PathSegment::new("{_private}");
        assert!(seg.is_param());
        assert_eq!(seg.as_param(), Some(&"_private"));
    }
}
