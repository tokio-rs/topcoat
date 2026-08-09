/// A static string held by reference so a view can record it in place.
///
/// A `&'static str` is two words wide, which does not fit into a view's
/// fixed-size instructions, so recording one stores it out of line in the
/// view's constants. A reference to a `&'static str` is one word wide,
/// because the pointer and length pair it refers to lives in the binary's
/// read-only data, so a view records it inline instead.
///
/// Wrap a string literal in this type to take that path:
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

impl PromotedStr {
    /// Wraps a promoted static string.
    #[inline]
    #[must_use]
    pub const fn new(value: &'static &'static str) -> Self {
        Self(value)
    }

    /// Returns the wrapped string.
    #[inline]
    #[must_use]
    pub const fn get(self) -> &'static str {
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
/// A string written as a literal goes one step further through
/// [`PromotedStr`], which also keeps it out of the view's constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticStr(pub &'static str);

impl StaticStr {
    /// Wraps a static string.
    #[inline]
    #[must_use]
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    /// Returns the wrapped string.
    #[inline]
    #[must_use]
    pub const fn get(self) -> &'static str {
        self.0
    }
}
