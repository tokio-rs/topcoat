use std::ops::Deref;

use topcoat_core::context::Cx;

use crate::runtime::{FmtHtml, Formatter};

/// A wrapper that marks its contents as already-safe HTML.
///
/// Use this only for trusted markup such as pre-rendered or sanitized HTML.
/// Passing untrusted input through this type defeats the runtime's escaping.
#[derive(Debug, Clone, PartialEq)]
pub struct Unescaped<T>(T);

impl<T> Unescaped<T> {
    /// Wraps `inner` as already-escaped content.
    ///
    /// # Safety (logical)
    ///
    /// The caller must ensure `inner` does not contain untrusted HTML.
    /// Misuse can lead to XSS vulnerabilities.
    #[inline]
    pub fn new_unchecked(inner: T) -> Self {
        Self(inner)
    }
}

impl FmtHtml for Unescaped<&str> {
    #[inline]
    fn fmt_html(&self, _cx: &Cx, f: &mut Formatter<'_>) {
        f.write_str_unescaped(self.0);
    }
}

impl FmtHtml for Unescaped<String> {
    #[inline]
    fn fmt_html(&self, _cx: &Cx, f: &mut Formatter<'_>) {
        f.write_str_unescaped(&self.0);
    }
}

impl<T> Deref for Unescaped<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
