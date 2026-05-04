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

impl<T> Fragment for &T
where
    T: Fragment + ?Sized,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        (*self).fmt(f)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        (*self).fmt_unescaped(f)
    }
}

impl Fragment for str {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        f.write_str(self)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        f.write_str_unescaped(self)
    }
}

impl Fragment for String {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        f.write_str(self)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        f.write_str_unescaped(self)
    }
}

impl<T> Fragment for Option<T>
where
    T: Fragment,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        if let Some(fragment) = self {
            fragment.fmt(f);
        }
    }
    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        if let Some(fragment) = self {
            fragment.fmt_unescaped(f);
        }
    }
}

impl Fragment for View {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) {
        // Views are guaranteed to already be escaped.
        self.fmt_unescaped(f);
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
        f.write_str_unescaped(&self.buf)
    }
}

macro_rules! impl_with_display {
    ($ty:ty) => {
        impl Fragment for $ty {
            #[inline]
            fn fmt(&self, f: &mut Formatter<'_>) {
                f.write_str(&self.to_string())
            }

            #[inline]
            fn fmt_unescaped(&self, f: &mut Formatter<'_>) {
                f.write_str_unescaped(&self.to_string())
            }
        }
    };
}

impl_with_display!(i8);
impl_with_display!(i16);
impl_with_display!(i32);
impl_with_display!(i64);
impl_with_display!(i128);
impl_with_display!(u8);
impl_with_display!(u16);
impl_with_display!(u32);
impl_with_display!(u64);
impl_with_display!(u128);
impl_with_display!(bool);

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
