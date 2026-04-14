use crate::runtime::{Formatter, view::View};

/// A piece of content that can be rendered into HTML.
///
/// Every `Fragment` provides two rendering paths:
///
/// - [`fmt`](Self::fmt) — the default, which escapes HTML-significant characters.
/// - [`fmt_unescaped`](Self::fmt_unescaped) — writes content verbatim, for trusted markup.
pub trait Fragment {
    /// Renders this fragment into the formatter, escaping by default.
    fn fmt(&self, f: &mut Formatter<'_>);
    /// Renders this fragment into the formatter without escaping.
    fn fmt_unescaped(&self, f: &mut Formatter<'_>);
}

impl<T> Fragment for T
where
    T: AsRef<str>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        f.write_str(self.as_ref())
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        f.write_str_unescaped(self.as_ref())
    }
}

impl Fragment for &View {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        f.write_str(&self.buf)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        f.write_str_unescaped(&self.buf)
    }
}

impl Fragment for View {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        <&View as Fragment>::fmt(&self, f)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        <&View as Fragment>::fmt_unescaped(&self, f)
    }
}

/// A wrapper that marks a fragment as already escaped / trusted.
///
/// When rendered, `Escaped<T>` bypasses escaping — both [`fmt`](Fragment::fmt)
/// and [`fmt_unescaped`](Fragment::fmt_unescaped) write the inner content
/// verbatim. This is useful for content that is known to be safe HTML, such as
/// the output of a previous render pass.
///
/// Constructed via [`new_unchecked`](Self::new_unchecked) to make the trust
/// decision explicit at the call site.
pub struct Escaped<T>(T);

impl<T> Escaped<T>
where
    T: Fragment,
{
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

impl<T> Fragment for Escaped<T>
where
    T: Fragment,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        self.fmt_unescaped(f)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        self.0.fmt_unescaped(f)
    }
}
