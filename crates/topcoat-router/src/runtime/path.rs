use std::{
    borrow::{Borrow, Cow},
    fmt::{Display, Write},
    ops::{AddAssign, Deref},
};

use ref_cast::{RefCastCustom, ref_cast_custom};

/// A borrowed route path, similar to [`std::path::Path`] but for URL paths.
///
/// A `Path` consists of `/`-separated segments, where each segment is one of:
/// - **`Static`**: a literal string (e.g. `users`)
/// - **`Param`**: a dynamic parameter in braces (e.g. `{id}`)
/// - **`CatchAll`**: a wildcard tail in braces with `*` (e.g. `{*rest}`)
/// - **`Group`**: a logical grouping in parentheses (e.g. `(auth)`), stripped when converting to a
///   `matchit` path
///
/// The root path `"/"` is normalized to an empty inner string. Use [`Path::new`] to
/// create a `&Path` from a string slice.
///
/// # Examples
///
/// ```
/// use topcoat_router::runtime::Path;
///
/// let path = Path::new("/users/(group)/{id}");
/// assert_eq!(path.segments().count(), 3);
/// assert_eq!(path.to_matchit_path(), "/users/{id}");
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
    ///
    /// This is the panicking counterpart of [`from_str`](Path::from_str).
    /// Because it is a `const fn`, malformed paths handed to the routing macros
    /// are rejected at compile time.
    ///
    /// # Panics
    ///
    /// Panics if `s` is not a well-formed path; see [`PathError`] for the
    /// conditions that are rejected.
    #[must_use]
    pub const fn new(s: &str) -> &Self {
        match Self::from_str(s) {
            Ok(path) => path,
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Creates a `&Path` from a string slice, validating its segments.
    ///
    /// The root path `"/"` is normalized to an empty inner representation. Every
    /// other path must be a sequence of `/`-prefixed, non-empty segments, each a
    /// valid [`PathSegment`]. Returns [`PathError`] if `s` is malformed.
    ///
    /// # Errors
    ///
    /// Returns [`PathError`] if `s` is not a valid path: it must be empty, be
    /// the root `"/"`, or be a sequence of `/`-prefixed valid segments.
    #[allow(clippy::should_implement_trait)]
    pub const fn from_str(s: &str) -> Result<&Self, PathError> {
        let s = match s.as_bytes() {
            [b'/'] => "",
            _ => s,
        };
        let bytes = s.as_bytes();
        let len = bytes.len();
        // The root path is empty and has no segments to validate.
        if len == 0 {
            return Ok(Self::new_unchecked(s));
        }
        if bytes[0] != b'/' {
            return Err(PathError::MissingLeadingSlash);
        }
        // Walk the `/`-separated segments, validating each `bytes[start..end)`.
        let mut start = 1;
        let mut i = 1;
        while i <= len {
            if i == len || bytes[i] == b'/' {
                if let Err(err) = validate_segment(bytes, start, i) {
                    return Err(err);
                }
                start = i + 1;
            }
            i += 1;
        }
        Ok(Self::new_unchecked(s))
    }

    /// Creates a `&Path` from a string slice without validating or normalizing it.
    ///
    /// This is a zero-cost reference cast. Unlike [`new`](Path::new), it
    /// performs no segment validation and does *not* normalize the root path `"/"`
    /// to an empty inner string. The caller must pass an already-valid, normalized
    /// path string (for example one obtained from another `Path`); passing
    /// anything else yields a `Path` that misbehaves when its segments are read.
    #[ref_cast_custom]
    #[must_use]
    pub const fn new_unchecked(s: &str) -> &Self;

    /// Returns an iterator over the [`PathSegment`]s of this path.
    ///
    /// The root path yields zero segments.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::runtime::{Path, PathSegment};
    ///
    /// let path = Path::new("/users/{id}/(auth)");
    /// let segs: Vec<_> = path.segments().collect();
    /// assert_eq!(
    ///     segs,
    ///     vec![
    ///         PathSegment::Static("users"),
    ///         PathSegment::Param("id"),
    ///         PathSegment::Group("auth"),
    ///     ]
    /// );
    /// ```
    pub fn segments(&self) -> impl Iterator<Item = PathSegment<'_>> {
        // The path was validated on construction, so its segments need no
        // re-validation here.
        self.inner
            .split('/')
            .skip(1)
            .map(PathSegment::new_unchecked)
    }

    /// Converts this path to a `matchit`-compatible route string, stripping group
    /// segments.
    ///
    /// Group segments (e.g. `(auth)`) are used for layout matching but are not
    /// part of the URL that the router matches against. This method removes them
    /// and returns the remaining path.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::runtime::Path;
    ///
    /// let path = Path::new("/(auth)/dashboard/{id}");
    /// assert_eq!(path.to_matchit_path(), "/dashboard/{id}");
    ///
    /// let root = Path::new("/");
    /// assert_eq!(root.to_matchit_path(), "/");
    ///
    /// // A path made up entirely of group segments collapses to the root URL,
    /// // e.g. a page in a `(marketing)` group that should serve `/`.
    /// let group_root = Path::new("/(marketing)");
    /// assert_eq!(group_root.to_matchit_path(), "/");
    /// ```
    #[must_use]
    pub fn to_matchit_path(&self) -> Cow<'static, str> {
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
        // result back to "/": matchit rejects route paths that don't start with "/".
        if stripped.is_empty() {
            return Cow::Borrowed("/");
        }
        Cow::Owned(stripped)
    }

    /// Returns `true` if this path starts with the given prefix path.
    ///
    /// Comparison is done segment-by-segment using [`PathSegment`] equality.
    /// This is used to determine which layouts apply to a given page: a layout
    /// at `"/settings"` matches any page whose path starts with `/settings`.
    ///
    /// Group segments are included in the comparison.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::runtime::Path;
    ///
    /// let path = Path::new("/users/{id}/posts");
    /// assert!(path.starts_with(Path::new("/users/{id}")));
    /// assert!(!path.starts_with(Path::new("/posts/{id}")));
    /// ```
    #[must_use]
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
    #[must_use]
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

/// The reason a string could not be parsed into a [`Path`] by
/// [`Path::from_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathError {
    /// The path was non-empty but did not start with `/`.
    MissingLeadingSlash,
    /// A segment was empty, as produced by a trailing or doubled `/`.
    EmptySegment,
    /// A `{` parameter or catch-all segment was missing its closing `}`.
    MissingClosingBrace,
    /// A `(` group segment was missing its closing `)`.
    MissingClosingParen,
    /// A static segment contained a `{`, `}`, `(`, or `)`.
    UnexpectedBracket,
    /// A param, catch-all, or group name was empty.
    EmptyName,
    /// A name did not start with an ASCII letter or `_`.
    InvalidNameStart,
    /// A name contained a character other than an ASCII alphanumeric or `_`.
    InvalidNameChar,
}

impl PathError {
    /// A human-readable description of the error.
    const fn message(self) -> &'static str {
        match self {
            Self::MissingLeadingSlash => "invalid path: must be empty or start with `/`",
            Self::EmptySegment => "invalid path: empty segment",
            Self::MissingClosingBrace => "invalid path: missing closing `}`",
            Self::MissingClosingParen => "invalid path: missing closing `)`",
            Self::UnexpectedBracket => "invalid path: unexpected bracket in static segment",
            Self::EmptyName => "invalid path: segment name must not be empty",
            Self::InvalidNameStart => {
                "invalid path: segment name must start with a letter or underscore"
            }
            Self::InvalidNameChar => "invalid path: segment name contains an invalid character",
        }
    }
}

impl Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for PathError {}

/// An owned route path, similar to [`std::path::PathBuf`] but for URL paths.
///
/// `PathBuf` is the owned counterpart of [`Path`]. It can be built incrementally
/// by adding [`PathSegment`]s with `+=`, or collected from an iterator of segments.
///
/// # Examples
///
/// ```
/// use topcoat_router::runtime::{PathBuf, PathSegment};
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
    #[must_use]
    pub fn new() -> Self {
        PathBuf::default()
    }
}

impl Borrow<Path> for PathBuf {
    fn borrow(&self) -> &Path {
        // A `PathBuf` only ever holds a valid path, so skip re-validation.
        Path::new_unchecked(&self.inner)
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        // A `PathBuf` only ever holds a valid path, so skip re-validation.
        Path::new_unchecked(&self.inner)
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
    /// This is the panicking counterpart of [`from_str`](PathSegment::from_str).
    ///
    /// # Panics
    ///
    /// Panics if `s` is not a well-formed segment; see [`PathError`] for the
    /// conditions that are rejected.
    #[must_use]
    pub fn new(s: &'a str) -> Self {
        match Self::from_str(s) {
            Ok(segment) => segment,
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Parses a single path segment string into a [`PathSegment`], validating it.
    ///
    /// Returns [`PathError`] if `s` is not a well-formed segment: an empty string,
    /// a `{...}`/`(...)` segment missing its closing bracket, a static segment that
    /// contains a bracket, or a name that is not a valid identifier.
    ///
    /// # Errors
    ///
    /// Returns [`PathError`] if `s` is not a well-formed segment.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &'a str) -> Result<Self, PathError> {
        // Validate first, then extract the variant from the now-known-valid input.
        validate_segment(s.as_bytes(), 0, s.len())?;
        Ok(Self::new_unchecked(s))
    }

    /// Parses a single path segment string into a [`PathSegment`] without
    /// validating it.
    ///
    /// Unlike [`new`](PathSegment::new) and [`from_str`](PathSegment::from_str),
    /// this performs no validation; the caller must pass an already-valid segment
    /// (for example one produced by [`Path::segments`]). A malformed input is
    /// parsed on a best-effort basis and yields a nonsensical segment rather than
    /// an error.
    #[must_use]
    pub fn new_unchecked(s: &'a str) -> Self {
        if let Some(inner) = s.strip_prefix('{') {
            let inner = inner.strip_suffix('}').unwrap_or(inner);
            match inner.strip_prefix('*') {
                Some(name) => PathSegment::CatchAll(name),
                None => PathSegment::Param(inner),
            }
        } else if let Some(inner) = s.strip_prefix('(') {
            PathSegment::Group(inner.strip_suffix(')').unwrap_or(inner))
        } else {
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
    #[must_use]
    pub fn as_static(&self) -> Option<&&'a str> {
        if let Self::Static(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the inner string if this is a [`Group`](PathSegment::Group) segment.
    #[must_use]
    pub fn as_group(&self) -> Option<&&'a str> {
        if let Self::Group(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the inner string if this is a [`Param`](PathSegment::Param) segment.
    #[must_use]
    pub fn as_param(&self) -> Option<&&'a str> {
        if let Self::Param(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the inner string if this is a [`CatchAll`](PathSegment::CatchAll) segment.
    #[must_use]
    pub fn as_catch_all(&self) -> Option<&&'a str> {
        if let Self::CatchAll(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

/// Validates a single segment `bytes[start..end)` of a [`Path`]. Operates on
/// bytes (rather than a `&str` subslice) so it can run in the `const` context of
/// [`Path::from_str`], and is shared with [`PathSegment::from_str`].
const fn validate_segment(bytes: &[u8], start: usize, end: usize) -> Result<(), PathError> {
    if start >= end {
        return Err(PathError::EmptySegment);
    }
    match bytes[start] {
        b'{' => {
            if bytes[end - 1] != b'}' {
                return Err(PathError::MissingClosingBrace);
            }
            // The name sits between the braces; a leading `*` marks a catch-all.
            let mut name_start = start + 1;
            let name_end = end - 1;
            if name_start < name_end && bytes[name_start] == b'*' {
                name_start += 1;
            }
            validate_ident(bytes, name_start, name_end)
        }
        b'(' => {
            if bytes[end - 1] != b')' {
                return Err(PathError::MissingClosingParen);
            }
            validate_ident(bytes, start + 1, end - 1)
        }
        _ => {
            // A static segment must not contain any of the reserved brackets.
            let mut i = start;
            while i < end {
                match bytes[i] {
                    b'{' | b'}' | b'(' | b')' => return Err(PathError::UnexpectedBracket),
                    _ => {}
                }
                i += 1;
            }
            Ok(())
        }
    }
}

/// Validates that `bytes[start..end)` is a valid identifier: non-empty, starting
/// with an ASCII letter or `_`, and otherwise only ASCII alphanumerics or `_`.
const fn validate_ident(bytes: &[u8], start: usize, end: usize) -> Result<(), PathError> {
    if start >= end {
        return Err(PathError::EmptyName);
    }
    let first = bytes[start];
    if !first.is_ascii_alphabetic() && first != b'_' {
        return Err(PathError::InvalidNameStart);
    }
    let mut i = start + 1;
    while i < end {
        let ch = bytes[i];
        if !ch.is_ascii_alphanumeric() && ch != b'_' {
            return Err(PathError::InvalidNameChar);
        }
        i += 1;
    }
    Ok(())
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

    // -- Path --

    #[test]
    fn path_root_slash_normalized() {
        let path = Path::new("/");
        assert_eq!(&path.inner, "");
        assert_eq!(path.to_matchit_path(), "/");
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
    fn path_to_matchit_strips_groups() {
        let path = Path::new("/(auth)/dashboard/{id}");
        assert_eq!(path.to_matchit_path(), "/dashboard/{id}");
    }

    #[test]
    fn path_to_matchit_empty() {
        let path = Path::new("");
        assert_eq!(path.to_matchit_path(), "/");
    }

    #[test]
    fn path_to_matchit_group_only_is_root() {
        // A page inside a route group that should serve `/`.
        assert_eq!(Path::new("/(marketing)").to_matchit_path(), "/");
        // Nested groups collapse the same way.
        assert_eq!(Path::new("/(a)/(b)").to_matchit_path(), "/");
    }

    #[test]
    fn path_to_matchit_no_groups() {
        let path = Path::new("/users/{id}");
        assert_eq!(path.to_matchit_path(), "/users/{id}");
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

    // -- Path validation --

    #[test]
    fn from_str_accepts_valid_paths() {
        for input in [
            "",
            "/",
            "/users",
            "/users/{id}",
            "/users/{id}/posts/{*rest}",
            "/(auth)/dashboard/{user_id}",
            "/{_private}",
        ] {
            assert!(Path::from_str(input).is_ok(), "rejected `{input}`");
        }
    }

    #[test]
    fn from_str_reports_errors() {
        use PathError::*;
        let cases = [
            ("users", MissingLeadingSlash),
            ("/users/", EmptySegment),
            ("/users//posts", EmptySegment),
            ("/foo{bar}", UnexpectedBracket),
            ("/{id", MissingClosingBrace),
            ("/(auth", MissingClosingParen),
            ("/{}", EmptyName),
            ("/{*}", EmptyName),
            ("/{0id}", InvalidNameStart),
            ("/{id-name}", InvalidNameChar),
            ("/(my-group)", InvalidNameChar),
        ];
        for (input, expected) in cases {
            assert_eq!(Path::from_str(input), Err(expected), "for `{input}`");
        }
    }

    #[test]
    fn new_validates_in_const_context() {
        // Compiles only because the path is valid; a malformed literal here would
        // be a compile-time error from the panic in `new`.
        const PATH: &Path = Path::new("/users/{id}/(auth)");
        assert_eq!(PATH.segments().count(), 3);
    }

    #[test]
    #[should_panic(expected = "unexpected bracket")]
    fn new_panics_on_invalid() {
        let _ = Path::new("/foo{bar}");
    }

    // -- PathBuf --

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

    // -- PathSegment --

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
        let _ = PathSegment::new("{id");
    }

    #[test]
    #[should_panic(expected = "missing closing `)`")]
    fn group_missing_close() {
        let _ = PathSegment::new("(auth");
    }

    #[test]
    #[should_panic(expected = "empty segment")]
    fn empty_segment() {
        let _ = PathSegment::new("");
    }

    #[test]
    #[should_panic(expected = "unexpected bracket")]
    fn static_with_braces() {
        let _ = PathSegment::new("foo{bar}");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn param_empty_name() {
        let _ = PathSegment::new("{}");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn group_empty_name() {
        let _ = PathSegment::new("()");
    }

    #[test]
    #[should_panic(expected = "name must not be empty")]
    fn catch_all_empty_name() {
        let _ = PathSegment::new("{*}");
    }

    #[test]
    #[should_panic(expected = "must start with a letter or underscore")]
    fn param_invalid_start() {
        let _ = PathSegment::new("{0id}");
    }

    #[test]
    #[should_panic(expected = "contains an invalid character")]
    fn param_invalid_char() {
        let _ = PathSegment::new("{id-name}");
    }

    #[test]
    #[should_panic(expected = "must start with a letter or underscore")]
    fn group_invalid_start() {
        let _ = PathSegment::new("(0auth)");
    }

    #[test]
    #[should_panic(expected = "contains an invalid character")]
    fn group_invalid_char() {
        let _ = PathSegment::new("(my-group)");
    }

    #[test]
    fn underscore_leading_ident() {
        let seg = PathSegment::new("{_private}");
        assert!(seg.is_param());
        assert_eq!(seg.as_param(), Some(&"_private"));
    }
}
