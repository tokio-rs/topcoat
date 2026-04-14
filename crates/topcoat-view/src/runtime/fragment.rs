use crate::runtime::{Formatter, view::View};

pub trait Fragment {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result;
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) -> std::fmt::Result;
}

impl<T> Fragment for T
where
    T: AsRef<str>,
{
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str_unescaped(self.as_ref())
    }
}

impl Fragment for &View {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.buf)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str_unescaped(&self.buf)
    }
}

impl Fragment for View {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <&View as Fragment>::fmt(&self, f)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        <&View as Fragment>::fmt_unescaped(&self, f)
    }
}

pub struct Escaped<T>(T);

impl<T> Escaped<T>
where
    T: Fragment,
{
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
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.fmt_unescaped(f)
    }

    #[inline]
    fn fmt_unescaped(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt_unescaped(f)
    }
}
