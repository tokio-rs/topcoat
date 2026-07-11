use std::ops::Deref;

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
