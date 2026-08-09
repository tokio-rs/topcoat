use std::ops::Deref;

/// A static string held by reference so a view can record it in place.
///
/// This is the most efficient way to pass a `&'static str` to a `view!`.
/// Use it to optimize your rendering, for example for static class strings.
///
/// ```rust
/// # use topcoat::view::{PromotedStr, component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result {
/// view! {
///     <div>(PromotedStr(&"hello"))</div>
/// }
/// # }
/// ```
///
/// The leading `&` is what makes this work: Rust promotes a reference to a
/// constant into the binary's read-only data. Only a constant can be
/// promoted, so a string that is only known at run time goes through
/// [`StaticStr`] instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromotedStr(pub &'static &'static str);

impl Deref for PromotedStr {
    type Target = &'static str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A static string a view records without copying it.
///
/// The `&str` implementations copy their contents into the view, since the
/// view can outlive the borrow. A `&'static str` outlives every view, so
/// wrapping one in this type records the string as is:
///
/// ```rust
/// # use topcoat::view::{StaticStr, component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result {
/// # let name: &'static str = "hello";
/// view! {
///     <div>(StaticStr(name))</div>
/// }
/// # }
/// ```
///
/// A string written as a literal can be further optimized by using [`PromotedStr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticStr(pub &'static str);

impl Deref for StaticStr {
    type Target = &'static str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A wrapper that marks its contents as already-safe HTML.
///
/// Use this only for trusted markup such as pre-rendered or sanitized HTML.
/// Passing untrusted input through this type defeats the runtime's escaping.
#[derive(Debug, Clone, PartialEq)]
pub struct Unescaped<T>(pub(crate) T);

impl<T> Unescaped<T> {
    /// Wraps `inner` as already-escaped content.
    ///
    /// # Safety (logical)
    ///
    /// The caller must ensure `inner` does not contain untrusted HTML.
    /// Misuse can lead to XSS vulnerabilities.
    #[inline]
    pub const fn new_unchecked(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Deref for Unescaped<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
