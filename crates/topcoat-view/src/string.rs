use std::ops::Deref;

/// A reference to a static string that a view can store without copying.
///
/// Use this when interpolating a string constant into a view:
///
/// ```rust
/// # use topcoat::view::{PromotedStr, View, component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result<impl View> {
/// Ok(view! {
///     <div>(PromotedStr(&"hello"))</div>
/// })
/// # }
/// ```
///
/// The leading `&` borrows the constant for the `'static` lifetime. Use
/// [`StaticStr`] for a static string selected at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
/// Wrap a `&'static str` in this type to avoid the copy made when a view
/// captures an ordinary `&str`:
///
/// ```rust
/// # use topcoat::view::{StaticStr, View, component, view};
/// # #[component]
/// # async fn example() -> topcoat::Result<impl View> {
/// # let name: &'static str = "hello";
/// Ok(view! {
///     <div>(StaticStr(name))</div>
/// })
/// # }
/// ```
///
/// A string written as a literal can be further optimized by using [`PromotedStr`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
