use std::ops::Deref;

use topcoat_core::context::Cx;

use crate::runtime::{Formatter, Fragment, IntoViewPart, ViewPart};

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

impl Fragment for Unescaped<&str> {
    #[inline]
    fn fmt(&self, _cx: &Cx, f: &mut Formatter<'_>) {
        f.write_str_unescaped(self.0);
    }
}

impl Fragment for Unescaped<String> {
    #[inline]
    fn fmt(&self, _cx: &Cx, f: &mut Formatter<'_>) {
        f.write_str_unescaped(&self.0);
    }
}

impl<T> Deref for Unescaped<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoViewPart for Unescaped<&str> {
    fn into_view_part(self) -> ViewPart {
        ViewPart::UnescapedString(Unescaped(self.0.to_owned()))
    }
}

impl IntoViewPart for Unescaped<String> {
    fn into_view_part(self) -> ViewPart {
        ViewPart::UnescapedString(self)
    }
}
