use std::{iter::once, ops::Deref};

use topcoat_core::context::Cx;

use crate::runtime::{Formatter, Fragment, IntoViewParts, ViewPart};

/// A wrapper that marks its contents as already-safe HTML.
///
/// When an `Unescaped<T>` is rendered, its inner value is written verbatim
/// using [`Formatter::write_str_unescaped`](crate::runtime::Formatter::write_str_unescaped)
/// instead of going through the usual escaping path. This is the escape
/// hatch for inserting trusted markup (pre-rendered HTML, sanitized
/// fragments, server-generated tags) into a [`View`](crate::runtime::View).
///
/// Construct an `Unescaped` with [`new_unchecked`](Self::new_unchecked); the
/// name reflects that the caller is asserting the content is safe to emit
/// without escaping. Passing untrusted input through this type defeats the
/// runtime's XSS protection.
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

impl IntoViewParts for Unescaped<&'static str> {
    fn into_view_part(self) -> impl Iterator<Item = ViewPart> {
        once(ViewPart::UnescapedStaticStr(self))
    }
}

impl IntoViewParts for Unescaped<String> {
    fn into_view_part(self) -> impl Iterator<Item = ViewPart> {
        once(ViewPart::UnescapedString(self))
    }
}
