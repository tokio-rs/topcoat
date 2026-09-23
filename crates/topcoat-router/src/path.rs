use std::{
    borrow::{Borrow, Cow},
    fmt::{Display, Write},
    iter::FusedIterator,
    mem,
    ops::{AddAssign, Deref},
};

use ref_cast::{RefCastCustom, ref_cast_custom};

/// A borrowed route path pattern, like [`std::path::Path`] but for URL paths.
///
/// A `Path` is a list of `/`-separated [`PathSegment`]s. Each segment is one
/// of:
///
/// - a static segment, like `users`, that matches itself,
/// - a parameter, like `{id}`, that matches any one segment,
/// - a catch-all, like `{*rest}`, that matches the rest of the URL,
/// - a group, like `(auth)`, that is not part of the URL but lets layouts and layers apply to only
///   some routes.
///
/// A trailing `/` is part of the path: `/users/` and `/users` are different
/// paths. The trailing slash is an empty static segment at the end. No other
/// segment may be empty.
///
/// Create a `&Path` from a string with [`Path::new`], or build an owned
/// [`PathBuf`].
///
/// # Examples
///
/// ```
/// use topcoat_router::{Path, PathSegment};
///
/// let path = Path::new("/users/(group)/{id}");
/// assert_eq!(path.segments().count(), 3);
/// assert_eq!(path.to_matchit_path(), "/users/{id}");
///
/// let slashed = Path::new("/users/");
/// assert_eq!(slashed.segments().last(), Some(PathSegment::Static("")));
/// assert_eq!(slashed.to_matchit_path(), "/users/");
/// ```
#[derive(Debug, PartialEq, Eq, Hash, RefCastCustom)]
#[repr(transparent)]
pub struct Path {
    inner: str,
}

impl Path {
    /// The root path `/`.
    pub const ROOT: &Path = Path::new("/");

    /// Creates a `&Path` from a string.
    ///
    /// This is the panicking version of [`from_str`](Path::from_str). It is a
    /// `const fn`, so a malformed path in a constant is rejected at compile
    /// time.
    ///
    /// # Panics
    ///
    /// Panics if `s` is not a well-formed path; see [`PathError`] for the
    /// conditions that are rejected.
    #[must_use]
    #[track_caller]
    pub const fn new(s: &str) -> &Self {
        match Self::from_str(s) {
            Ok(path) => path,
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Creates a `&Path` from a string, checking that it is well formed.
    ///
    /// A path is either empty, the root `/`, or a list of segments that each
    /// start with `/` and are valid [`PathSegment`]s. Only the last segment
    /// may be empty, which is how a trailing `/` is written. The empty string
    /// and `/` both give the root path.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if `s` is not a well-formed path.
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
        // The last segment may be empty, as long as it is not also the first:
        // that would be `//`, which is not the root with a trailing slash.
        let mut start = 1;
        let mut i = 1;
        while i <= len {
            if i == len || bytes[i] == b'/' {
                let trailing_slash = i == len && start == len && start > 1;
                if !trailing_slash && let Err(err) = validate_segment(bytes, start, i) {
                    return Err(err);
                }
                start = i + 1;
            }
            i += 1;
        }
        Ok(Self::new_unchecked(s))
    }

    /// Creates a `&Path` from a string without checking it.
    ///
    /// Unlike [`new`](Path::new), this does not turn `"/"` into the empty
    /// string that backs the root path. Only pass a string that came from
    /// another `Path`, such as the result of [`as_str`](Path::as_str). Any
    /// other string can give a `Path` whose segments read wrong.
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
    /// use topcoat_router::{Path, PathSegment};
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
    pub fn segments(&self) -> PathSegments<'_> {
        PathSegments::new(self)
    }

    /// Returns the path the router matches URLs against: this path without its
    /// group segments.
    ///
    /// A path with no segments left, like the root or a path of only groups,
    /// becomes `/`.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::Path;
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

    /// Returns `true` if the segments of `other` are the first segments of
    /// this path.
    ///
    /// Segments are compared whole, so `/users` does not start with `/use`.
    /// Group segments and parameter names are compared too. The root path is
    /// a prefix of every path.
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
    #[must_use]
    pub fn starts_with(&self, other: &Path) -> bool {
        if self.inner.len() < other.inner.len() {
            return false;
        }
        self.segments().zip(other.segments()).all(|(a, b)| a == b)
    }

    /// Returns a new path with the segments of `other` added to the end of
    /// this path.
    ///
    /// Joining the root path on either side gives the other path unchanged. A
    /// trailing slash on this path is dropped when segments follow it.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::Path;
    ///
    /// let base = Path::new("/settings");
    /// assert_eq!(base.join(Path::new("/export")).as_str(), "/settings/export");
    /// assert_eq!(base.join(Path::ROOT).as_str(), "/settings");
    /// assert_eq!(Path::ROOT.join(base).as_str(), "/settings");
    /// assert_eq!(
    ///     Path::new("/settings/").join(Path::new("/export")).as_str(),
    ///     "/settings/export"
    /// );
    /// ```
    #[must_use]
    pub fn join(&self, other: &Path) -> PathBuf {
        let mut buf = self.to_owned();
        buf += other;
        buf
    }

    /// Returns `true` if the URL path `url` matches this path pattern.
    ///
    /// - A static segment must equal the URL segment.
    /// - A parameter matches any one non-empty URL segment.
    /// - A catch-all matches the rest of the URL, including `/` separators, but not an empty rest.
    /// - A group segment is skipped, since it is not part of the URL.
    ///
    /// A trailing `/` must match too: a path without one does not match a
    /// URL with one, and the other way around. The URL is compared as it is,
    /// without percent-decoding.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::Path;
    ///
    /// let path = Path::new("/users/{id}/posts");
    /// assert!(path.matches("/users/42/posts"));
    /// assert!(!path.matches("/users/42"));
    ///
    /// // Group segments are ignored.
    /// assert!(Path::new("/(auth)/dashboard").matches("/dashboard"));
    ///
    /// // A catch-all matches the remainder of the URL.
    /// assert!(Path::new("/files/{*rest}").matches("/files/a/b/c"));
    ///
    /// // A trailing slash has to match.
    /// assert!(Path::new("/users/").matches("/users/"));
    /// assert!(!Path::new("/users/").matches("/users"));
    /// assert!(!Path::new("/users").matches("/users/"));
    /// ```
    #[must_use]
    pub fn matches(&self, url: &str) -> bool {
        // Splits the `/`-separated URL body into its first segment and the
        // remainder after the separator, e.g. "users/42" into ("users",
        // Some("42")) and "users" into ("users", None): the remainder is `None`
        // once the body is used up without a separator left over.
        fn first_segment(rest: &str) -> (&str, Option<&str>) {
            match rest.split_once('/') {
                Some((head, tail)) => (head, Some(tail)),
                None => (rest, None),
            }
        }

        // Drop a single leading `/`; what remains is the `/`-separated body,
        // e.g. "users/42/posts". The root URL "/" has nothing left to consume,
        // while a trailing separator leaves an empty body behind.
        let body = url.strip_prefix('/').unwrap_or(url);
        let mut rest = (!body.is_empty()).then_some(body);
        for segment in self.segments() {
            match segment {
                // Groups exist only for layout matching and never appear in a URL.
                PathSegment::Group(_) => {}
                // A trailing slash matches when the URL ended in a separator
                // with nothing after it.
                PathSegment::Static("") => return rest == Some(""),
                PathSegment::Static(expected) => match rest.map(first_segment) {
                    Some((head, tail)) if head == expected => rest = tail,
                    _ => return false,
                },
                // A parameter matches any single non-empty segment. An empty
                // one (as in `/users//`) never routes, so reject it here too.
                PathSegment::Param(_) => match rest.map(first_segment) {
                    Some((head, tail)) if !head.is_empty() => rest = tail,
                    _ => return false,
                },
                // A catch-all swallows the whole remainder, so nothing can
                // follow it and there is never leftover URL to reject.
                PathSegment::CatchAll(_) => return rest.is_some_and(|rest| !rest.is_empty()),
            }
        }
        // Every route segment matched; the URL must also be used up.
        rest.is_none()
    }

    /// Returns this path as a string.
    ///
    /// The root path is the empty string, not `"/"`.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::Path;
    ///
    /// assert_eq!(Path::new("/users/{id}").as_str(), "/users/{id}");
    /// assert_eq!(Path::new("/").as_str(), "");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.inner
    }

    /// Returns `true` if this path ends in a `/`, which means its last segment
    /// is empty. The root path does not count.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::Path;
    ///
    /// assert!(Path::new("/users/").has_trailing_slash());
    /// assert!(!Path::new("/users").has_trailing_slash());
    /// assert!(!Path::new("/").has_trailing_slash());
    /// ```
    #[must_use]
    pub fn has_trailing_slash(&self) -> bool {
        self.inner.ends_with('/')
    }

    /// Returns the length of [`as_str`](Path::as_str) in bytes.
    ///
    /// The root path has length zero.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if this path has no segments, which means it is the root
    /// path `/`.
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

impl<'a> From<&'a Path> for Cow<'a, Path> {
    fn from(value: &'a Path) -> Self {
        Self::Borrowed(value)
    }
}

/// An iterator over the [`PathSegment`]s of a [`Path`], created by
/// [`Path::segments`].
#[derive(Debug, Clone)]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub struct PathSegments<'path> {
    /// The `/`-separated body left to walk, without a leading `/`.
    rest: &'path str,
    /// Whether the body is used up. It is tracked separately because the last
    /// segment leaves `rest` empty, which is also how the root path starts out.
    done: bool,
}

impl<'path> PathSegments<'path> {
    fn new(path: &'path Path) -> Self {
        match path.inner.strip_prefix('/') {
            Some(rest) => Self { rest, done: false },
            // The root path is backed by the empty string and has no segments.
            None => Self {
                rest: "",
                done: true,
            },
        }
    }

    /// Marks the body as used up and returns what was left of it, the segment
    /// at whichever end the caller was reading.
    fn last_segment(&mut self) -> &'path str {
        self.done = true;
        mem::take(&mut self.rest)
    }
}

impl<'path> Iterator for PathSegments<'path> {
    type Item = PathSegment<'path>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let segment = match self.rest.split_once('/') {
            Some((segment, rest)) => {
                self.rest = rest;
                segment
            }
            None => self.last_segment(),
        };
        // The path was validated on construction, so its segments need no
        // re-validation here.
        Some(PathSegment::new_unchecked(segment))
    }
}

impl DoubleEndedIterator for PathSegments<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let segment = match self.rest.rsplit_once('/') {
            Some((rest, segment)) => {
                self.rest = rest;
                segment
            }
            None => self.last_segment(),
        };
        Some(PathSegment::new_unchecked(segment))
    }
}

impl FusedIterator for PathSegments<'_> {}

/// The reason a string is not a well-formed [`Path`] or [`PathSegment`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PathError {
    /// The path was non-empty but did not start with `/`.
    MissingLeadingSlash,
    /// A segment other than the last was empty, as in `/a//b`.
    EmptySegment,
    /// A `{` parameter or catch-all segment was missing its closing `}`.
    MissingClosingBrace,
    /// A `(` group segment was missing its closing `)`.
    MissingClosingParen,
    /// A static segment contained a `{`, `}`, `(`, or `)`.
    UnexpectedBracket,
    /// A parameter, catch-all, or group name was empty.
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

/// An owned route path pattern, like [`std::path::PathBuf`] but for URL
/// paths.
///
/// `PathBuf` is the owned version of [`Path`] and dereferences to it. Build
/// one by adding [`PathSegment`]s or whole paths with `+=`, or collect it
/// from an iterator of segments.
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
    /// Creates a `PathBuf` holding the root path `/`.
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

impl From<PathBuf> for Cow<'static, Path> {
    fn from(value: PathBuf) -> Self {
        Self::Owned(value)
    }
}

impl Deref for PathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        // A `PathBuf` only ever holds a valid path, so skip re-validation.
        Path::new_unchecked(&self.inner)
    }
}

impl PathBuf {
    /// Drops a trailing slash so that appended segments do not produce an
    /// empty segment in the middle of the path.
    fn pop_trailing_slash(&mut self) {
        if self.inner.ends_with('/') {
            self.inner.pop();
        }
    }
}

impl AddAssign<PathSegment<'_>> for PathBuf {
    fn add_assign(&mut self, rhs: PathSegment<'_>) {
        self.pop_trailing_slash();
        write!(self.inner, "/{rhs}").unwrap();
    }
}

impl AddAssign<&Path> for PathBuf {
    fn add_assign(&mut self, rhs: &Path) {
        if rhs.is_empty() {
            return;
        }
        // Both sides hold a validated path whose root is the empty string, so
        // appending the raw string yields a valid path again.
        self.pop_trailing_slash();
        self.inner.push_str(&rhs.inner);
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

/// A value that converts into a route [`Path`].
///
/// APIs that take a path accept any `IntoPath` value. A `&'static str` is
/// parsed with [`Path::new`]. [`Path`], [`PathBuf`], and `Cow<'static, Path>`
/// values are used as they are.
pub trait IntoPath {
    /// Converts the value into a route path.
    ///
    /// # Panics
    ///
    /// Panics if the value is a string that is not a well-formed path.
    #[track_caller]
    fn into_path(self) -> Cow<'static, Path>;
}

impl IntoPath for &'static str {
    #[track_caller]
    fn into_path(self) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new(self))
    }
}

impl IntoPath for &'static Path {
    fn into_path(self) -> Cow<'static, Path> {
        Cow::Borrowed(self)
    }
}

impl IntoPath for PathBuf {
    fn into_path(self) -> Cow<'static, Path> {
        Cow::Owned(self)
    }
}

impl IntoPath for Cow<'static, Path> {
    fn into_path(self) -> Cow<'static, Path> {
        self
    }
}

impl Display for PathBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

/// One segment of a route [`Path`].
///
/// | Syntax    | Variant    | Example   | Matches                                   |
/// |-----------|------------|-----------|-------------------------------------------|
/// | `name`    | `Static`   | `users`   | The same URL segment                      |
/// | `{name}`  | `Param`    | `{id}`    | Any one URL segment                       |
/// | `{*name}` | `CatchAll` | `{*path}` | The rest of the URL                       |
/// | `(name)`  | `Group`    | `(auth)`  | Nothing, it is not part of the URL        |
///
/// The names of parameters, catch-alls, and groups must start with an ASCII
/// letter or `_` and contain only ASCII letters, digits, and `_`. A static
/// segment must not contain `{`, `}`, `(`, or `)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment<'a> {
    /// A literal URL segment, like `users`.
    Static(&'a str),
    /// A group, like `(auth)`, that is not part of the URL.
    Group(&'a str),
    /// A parameter, like `{id}`, that matches one URL segment.
    Param(&'a str),
    /// A catch-all, like `{*rest}`, that matches the rest of the URL.
    CatchAll(&'a str),
}

impl<'a> PathSegment<'a> {
    /// Parses one path segment, like `users` or `{id}`.
    ///
    /// This is the panicking version of [`from_str`](PathSegment::from_str).
    ///
    /// # Panics
    ///
    /// Panics if `s` is not a well-formed segment; see [`PathError`] for the
    /// conditions that are rejected.
    #[must_use]
    #[track_caller]
    pub fn new(s: &'a str) -> Self {
        match Self::from_str(s) {
            Ok(segment) => segment,
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Parses one path segment, like `users` or `{id}`, checking that it is
    /// well formed.
    ///
    /// # Errors
    ///
    /// Returns a [`PathError`] if `s` is not a well-formed segment: it is
    /// empty, a `{` or `(` is not closed, a static segment contains a
    /// bracket, or a name is not valid.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &'a str) -> Result<Self, PathError> {
        // Validate first, then extract the variant from the now-known-valid input.
        validate_segment(s.as_bytes(), 0, s.len())?;
        Ok(Self::new_unchecked(s))
    }

    /// Parses one path segment without checking it.
    ///
    /// Only pass a segment that is known to be well formed. A malformed
    /// segment gives a meaningless result instead of an error.
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

    /// Returns the text if this is a [`Static`](PathSegment::Static) segment.
    #[must_use]
    pub fn as_static(&self) -> Option<&&'a str> {
        if let Self::Static(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the name if this is a [`Group`](PathSegment::Group) segment.
    #[must_use]
    pub fn as_group(&self) -> Option<&&'a str> {
        if let Self::Group(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the name of a parameter or catch-all segment, or `None` for
    /// other segments.
    ///
    /// # Examples
    ///
    /// ```
    /// use topcoat_router::PathSegment;
    ///
    /// assert_eq!(PathSegment::Param("id").param_name(), Some("id"));
    /// assert_eq!(PathSegment::CatchAll("rest").param_name(), Some("rest"));
    /// assert_eq!(PathSegment::Static("users").param_name(), None);
    /// ```
    #[must_use]
    pub fn param_name(&self) -> Option<&'a str> {
        match *self {
            Self::Param(name) | Self::CatchAll(name) => Some(name),
            Self::Static(_) | Self::Group(_) => None,
        }
    }

    /// Returns the name if this is a [`Param`](PathSegment::Param) segment.
    #[must_use]
    pub fn as_param(&self) -> Option<&&'a str> {
        if let Self::Param(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns the name if this is a [`CatchAll`](PathSegment::CatchAll)
    /// segment.
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
    fn path_segments_from_the_back() {
        let path = Path::new("/dashboard/{id}/(auth)");
        let segs: Vec<_> = path.segments().rev().collect();
        assert_eq!(
            segs,
            vec![
                PathSegment::Group("auth"),
                PathSegment::Param("id"),
                PathSegment::Static("dashboard"),
            ]
        );
    }

    #[test]
    fn path_segments_from_both_ends_meet_in_the_middle() {
        let path = Path::new("/a/b/c");
        let mut segments = path.segments();

        assert_eq!(segments.next(), Some(PathSegment::Static("a")));
        assert_eq!(segments.next_back(), Some(PathSegment::Static("c")));
        assert_eq!(segments.next(), Some(PathSegment::Static("b")));
        // Both ends are exhausted once they meet.
        assert_eq!(segments.next(), None);
        assert_eq!(segments.next_back(), None);
    }

    #[test]
    fn root_path_yields_no_segments_from_either_end() {
        let mut segments = Path::new("/").segments();

        assert_eq!(segments.next(), None);
        assert_eq!(segments.next_back(), None);
    }

    #[test]
    fn trailing_slash_is_an_empty_last_segment() {
        let path = Path::new("/users/{id}/");
        let segs: Vec<_> = path.segments().collect();
        assert_eq!(
            segs,
            vec![
                PathSegment::Static("users"),
                PathSegment::Param("id"),
                PathSegment::Static(""),
            ]
        );
        assert_eq!(path.segments().next_back(), Some(PathSegment::Static("")));
        assert_eq!(path.as_str(), "/users/{id}/");
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
    fn path_to_matchit_keeps_trailing_slash() {
        assert_eq!(Path::new("/users/").to_matchit_path(), "/users/");
        assert_eq!(
            Path::new("/(auth)/users/{id}/").to_matchit_path(),
            "/users/{id}/"
        );
        // A trailing slash after nothing but groups still addresses the root.
        assert_eq!(Path::new("/(marketing)/").to_matchit_path(), "/");
    }

    // -- join --

    #[test]
    fn join_appends_segments() {
        let joined = Path::new("/settings").join(Path::new("/(admin)/{id}"));
        assert_eq!(joined.as_str(), "/settings/(admin)/{id}");
        assert_eq!(joined.segments().count(), 3);
    }

    #[test]
    fn join_root_on_either_side_is_identity() {
        let path = Path::new("/settings");
        assert_eq!(&*path.join(Path::ROOT), path);
        assert_eq!(&*Path::ROOT.join(path), path);
        assert!(Path::ROOT.join(Path::ROOT).is_empty());
    }

    #[test]
    fn path_buf_add_assign_path() {
        let mut buf = PathBuf::new();
        buf += Path::new("/users");
        buf += PathSegment::Param("id");
        buf += Path::new("/posts");
        assert_eq!(buf.as_str(), "/users/{id}/posts");
    }

    #[test]
    fn join_drops_a_trailing_slash_when_segments_follow() {
        let base = Path::new("/settings/");
        assert_eq!(base.join(Path::new("/export")).as_str(), "/settings/export");
        assert_eq!(
            base.join(Path::new("/export/")).as_str(),
            "/settings/export/"
        );
        let mut buf = base.to_owned();
        buf += PathSegment::Param("id");
        assert_eq!(buf.as_str(), "/settings/{id}");
    }

    #[test]
    fn join_keeps_a_trailing_slash_when_nothing_follows() {
        let path = Path::new("/settings/");
        assert_eq!(&*path.join(Path::ROOT), path);
        assert_eq!(&*Path::ROOT.join(path), path);
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
    fn path_starts_with_rejects_partial_segment() {
        // `/admin` is a string prefix of `/administrator`, but not a whole
        // segment, so it must not count as a path prefix.
        assert!(!Path::new("/administrator").starts_with(Path::new("/admin")));
    }

    #[test]
    fn path_starts_with_includes_groups() {
        let path = Path::new("/(auth)/dashboard");
        assert!(path.starts_with(Path::new("/(auth)")));
        // Groups are part of the logical path, so `/dashboard` is not a prefix
        // of `/(auth)/dashboard` even though both serve the URL `/dashboard`.
        assert!(!path.starts_with(Path::new("/dashboard")));
    }

    #[test]
    fn path_starts_with_distinguishes_param_names() {
        let path = Path::new("/users/{id}/posts");
        assert!(path.starts_with(Path::new("/users/{id}")));
        assert!(!path.starts_with(Path::new("/users/{user_id}")));
    }

    #[test]
    fn path_starts_with_trailing_slash() {
        // A slashed path lies under its slash-less prefix, but a slashed
        // prefix only covers itself.
        assert!(Path::new("/users/").starts_with(Path::new("/users")));
        assert!(Path::new("/users/").starts_with(Path::new("/users/")));
        assert!(!Path::new("/users").starts_with(Path::new("/users/")));
        assert!(!Path::new("/users/posts").starts_with(Path::new("/users/")));
    }

    #[test]
    fn path_display() {
        let path = Path::new("/users/{id}");
        assert_eq!(path.to_string(), "/users/{id}");
    }

    // -- Path matching --

    #[test]
    fn matches_static_exact() {
        assert!(Path::new("/users/list").matches("/users/list"));
    }

    #[test]
    fn matches_static_mismatch() {
        assert!(!Path::new("/users/list").matches("/users/all"));
    }

    #[test]
    fn matches_rejects_partial_segment() {
        assert!(!Path::new("/admin").matches("/administrator"));
        assert!(!Path::new("/administrator").matches("/admin"));
    }

    #[test]
    fn matches_is_case_sensitive() {
        assert!(!Path::new("/admin").matches("/Admin"));
    }

    #[test]
    fn matches_rejects_empty_segments() {
        // Doubled slashes produce empty URL segments, which never route.
        assert!(!Path::new("/admin").matches("//admin"));
        assert!(!Path::new("/users/{id}").matches("/users//"));
        assert!(!Path::new("/users/{id}/posts").matches("/users//posts"));
    }

    #[test]
    fn matches_treats_percent_encoding_as_opaque() {
        // Matching happens on the raw URL, where `%2F` is an ordinary part of
        // a segment, not a separator.
        assert!(!Path::new("/admin/users").matches("/admin%2Fusers"));
        assert!(Path::new("/{page}").matches("/admin%2Fusers"));
    }

    #[test]
    fn matches_param_captures_any_segment() {
        let path = Path::new("/users/{id}/posts");
        assert!(path.matches("/users/42/posts"));
        assert!(path.matches("/users/anything/posts"));
    }

    #[test]
    fn matches_rejects_too_few_segments() {
        assert!(!Path::new("/users/{id}/posts").matches("/users/42"));
    }

    #[test]
    fn matches_rejects_trailing_segments() {
        assert!(!Path::new("/users/{id}").matches("/users/42/posts"));
    }

    #[test]
    fn matches_ignores_groups() {
        assert!(Path::new("/(auth)/dashboard").matches("/dashboard"));
        assert!(Path::new("/(a)/{id}/(b)").matches("/42"));
    }

    #[test]
    fn matches_root() {
        assert!(Path::new("/").matches("/"));
        assert!(!Path::new("/").matches("/anything"));
    }

    #[test]
    fn matches_group_only_path_is_root() {
        assert!(Path::new("/(marketing)").matches("/"));
    }

    #[test]
    fn matches_trailing_slash_exactly() {
        assert!(!Path::new("/users").matches("/users/"));
        assert!(Path::new("/users/").matches("/users/"));
        assert!(!Path::new("/users/").matches("/users"));
        assert!(!Path::new("/users/").matches("/users//"));
        assert!(!Path::new("/users/").matches("/users/posts"));
        assert!(Path::new("/users/{id}/").matches("/users/42/"));
        assert!(!Path::new("/users/{id}/").matches("/users/42"));
        assert!(!Path::new("/users/{id}").matches("/users/"));
    }

    #[test]
    fn matches_root_rejects_doubled_slash() {
        assert!(!Path::new("/").matches("//"));
    }

    #[test]
    fn matches_catch_all() {
        let path = Path::new("/files/{*rest}");
        assert!(path.matches("/files/a"));
        assert!(path.matches("/files/a/b/c"));
    }

    #[test]
    fn matches_catch_all_requires_a_segment() {
        assert!(!Path::new("/files/{*rest}").matches("/files"));
        assert!(!Path::new("/files/{*rest}").matches("/files/"));
    }

    #[test]
    fn matches_catch_all_swallows_empty_segments() {
        // The remainder after `/files/` is `/`, a non-empty capture.
        assert!(Path::new("/files/{*rest}").matches("/files//"));
    }

    #[test]
    fn matches_non_origin_form_urls() {
        // An asterisk-form request (`OPTIONS *`) matches no route path. An
        // empty authority-form path is equivalent to the root URL.
        assert!(!Path::new("/").matches("*"));
        assert!(!Path::new("/admin").matches("*"));
        assert!(Path::new("/").matches(""));
        assert!(!Path::new("/admin").matches(""));
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
            "/users/",
            "/users/{id}/",
            "/(marketing)/",
        ] {
            assert!(Path::from_str(input).is_ok(), "rejected `{input}`");
        }
    }

    #[test]
    fn from_str_reports_errors() {
        use PathError::*;
        let cases = [
            ("users", MissingLeadingSlash),
            ("//", EmptySegment),
            ("/users//", EmptySegment),
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
    fn pathbuf_add_assign_trailing_slash() {
        let mut buf = PathBuf::new();
        buf += PathSegment::Static("users");
        buf += PathSegment::Static("");
        assert_eq!(buf.to_string(), "/users/");
        assert_eq!(&*buf, Path::new("/users/"));
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
    fn only_param_and_catch_all_segments_capture() {
        assert_eq!(PathSegment::new("{id}").param_name(), Some("id"));
        assert_eq!(PathSegment::new("{*rest}").param_name(), Some("rest"));
        assert_eq!(PathSegment::new("users").param_name(), None);
        assert_eq!(PathSegment::new("(auth)").param_name(), None);
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
