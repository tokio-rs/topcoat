use std::ops::Deref;

/// A string literal held by reference, the cheapest string for a view to
/// record.
///
/// A view records only the reference, without copying the string. Use it
/// for string constants that render often, such as static class lists.
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
/// The leading `&` is required. Rust promotes a reference to a constant
/// into the binary's read-only data, which gives the `&'static &'static str`
/// this type holds. Only a constant can be promoted, so use [`StaticStr`] for
/// a `&'static str` that is not a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PromotedStr(pub &'static &'static str);

impl Deref for PromotedStr {
    type Target = &'static str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

/// A `&'static str` that a view records without copying it.
///
/// A plain `&str` is copied into the view, because the view can outlive the
/// borrow. A `&'static str` outlives every view, so wrapping it in this type
/// skips the copy:
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
/// For a string literal, [`PromotedStr`] is cheaper still.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaticStr(pub &'static str);

impl Deref for StaticStr {
    type Target = &'static str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A value that renders without escaping.
///
/// Wrap trusted markup, such as pre-rendered or sanitized HTML, in this type
/// to write it into a view verbatim. Create one with
/// [`new_unchecked`](Self::new_unchecked).
#[derive(Debug, Clone, PartialEq)]
pub struct Unescaped<T>(pub(crate) T);

impl<T> Unescaped<T> {
    /// Wraps `inner` so that it renders without escaping.
    ///
    /// The caller must make sure `inner` contains no untrusted input.
    /// Passing untrusted input skips the escaping that views apply to it and
    /// can lead to XSS vulnerabilities.
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
