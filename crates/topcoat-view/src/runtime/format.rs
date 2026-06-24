use std::{ops::Deref, rc::Rc, sync::Arc};

use topcoat_core::runtime::context::Cx;

/// An HTML-aware writer.
///
/// The escaped methods handle the five characters that are meaningful in HTML:
///
/// | Character | Escaped as |
/// |-----------|------------|
/// | `&`       | `&amp;`    |
/// | `<`       | `&lt;`     |
/// | `>`       | `&gt;`     |
/// | `"`       | `&quot;`   |
/// | `'`       | `&#x27;`   |
///
/// Use [`write_str`](Self::write_str) and [`write_char`](Self::write_char) for
/// dynamic text. Use the unescaped methods only for trusted markup.
pub struct Formatter<'a> {
    buf: &'a mut String,
}

impl<'a> Formatter<'a> {
    /// Creates a new `Formatter` that writes into the given destination.
    #[inline]
    pub fn new(buf: &'a mut String) -> Self {
        Self { buf }
    }

    /// Writes a single character, escaping it if it is HTML-significant.
    #[inline]
    pub fn write_char(&mut self, c: char) {
        match c {
            '&' => self.buf.push_str("&amp;"),
            '<' => self.buf.push_str("&lt;"),
            '>' => self.buf.push_str("&gt;"),
            '"' => self.buf.push_str("&quot;"),
            '\'' => self.buf.push_str("&#x27;"),
            _ => self.write_char_unescaped(c),
        }
    }

    /// Writes a single character without escaping. Use for trusted content only.
    #[inline]
    pub fn write_char_unescaped(&mut self, c: char) {
        self.buf.push(c);
    }

    /// Writes a string, escaping any HTML-significant characters.
    #[inline]
    pub fn write_str(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let mut last = 0;

        for (i, &b) in bytes.iter().enumerate() {
            let escape = match b {
                b'&' => "&amp;",
                b'<' => "&lt;",
                b'>' => "&gt;",
                b'"' => "&quot;",
                b'\'' => "&#x27;",
                _ => continue,
            };

            if last < i {
                self.buf.push_str(&s[last..i]);
            }
            self.buf.push_str(escape);
            last = i + 1;
        }

        if last < s.len() {
            self.buf.push_str(&s[last..]);
        }
    }

    /// Writes a string without escaping. Use for trusted markup like tags and attributes.
    #[inline]
    pub fn write_str_unescaped(&mut self, s: &str) {
        self.buf.push_str(s);
    }
}

/// A value that can render itself into a [`Formatter`].
///
/// Implement this for custom renderable types. Use the escaped formatter
/// methods for dynamic or user-provided text, and the unescaped methods only
/// for trusted markup.
pub trait FmtHtml {
    /// Writes into `f`, escaping content as appropriate.
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>);

    /// Returns a lower bound on the number of bytes this fragment will write.
    ///
    /// Used to pre-allocate the output buffer. Implementations should err on
    /// the side of under-estimating; over-estimates waste memory while
    /// under-estimates only cost an extra reallocation.
    #[inline]
    fn size_hint(&self) -> usize {
        0
    }
}

impl<T> FmtHtml for &T
where
    T: FmtHtml + ?Sized,
{
    #[inline]
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
        (*self).fmt_html(cx, f);
    }

    #[inline]
    fn size_hint(&self) -> usize {
        (*self).size_hint()
    }
}

impl FmtHtml for str {
    #[inline]
    fn fmt_html(&self, _cx: &Cx, f: &mut Formatter<'_>) {
        f.write_str(self);
    }

    #[inline]
    fn size_hint(&self) -> usize {
        self.len()
    }
}

impl FmtHtml for String {
    #[inline]
    fn fmt_html(&self, _cx: &Cx, f: &mut Formatter<'_>) {
        f.write_str(self);
    }

    #[inline]
    fn size_hint(&self) -> usize {
        self.len()
    }
}

impl<T> FmtHtml for Option<T>
where
    T: FmtHtml,
{
    #[inline]
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
        if let Some(fragment) = self {
            fragment.fmt_html(cx, f);
        }
    }

    #[inline]
    fn size_hint(&self) -> usize {
        match self {
            Some(inner) => inner.size_hint(),
            None => 0,
        }
    }
}

struct UnescapedDisplayAdapter<'a, 'b>(&'a mut Formatter<'b>);

impl core::fmt::Write for UnescapedDisplayAdapter<'_, '_> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.write_str_unescaped(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        self.0.write_char_unescaped(c);
        Ok(())
    }
}

macro_rules! impl_with_display {
    ($ty:ty) => {
        impl FmtHtml for $ty {
            #[inline]
            fn fmt_html(&self, _cx: &Cx, f: &mut Formatter<'_>) {
                use core::fmt::Write;
                let _ = write!(UnescapedDisplayAdapter(f), "{self}");
            }

            #[inline]
            fn size_hint(&self) -> usize {
                1
            }
        }
    };
}

impl_with_display!(i8);
impl_with_display!(i16);
impl_with_display!(i32);
impl_with_display!(i64);
impl_with_display!(i128);
impl_with_display!(isize);
impl_with_display!(u8);
impl_with_display!(u16);
impl_with_display!(u32);
impl_with_display!(u64);
impl_with_display!(u128);
impl_with_display!(usize);
impl_with_display!(f32);
impl_with_display!(f64);
impl_with_display!(bool);
impl_with_display!(char);

macro_rules! impl_smart_pointer {
    ($name:ident) => {
        impl<T> FmtHtml for $name<T>
        where
            T: FmtHtml + ?Sized,
        {
            #[inline]
            fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
                self.deref().fmt_html(cx, f);
            }

            #[inline]
            fn size_hint(&self) -> usize {
                self.deref().size_hint()
            }
        }
    };
}

impl_smart_pointer!(Box);
impl_smart_pointer!(Rc);
impl_smart_pointer!(Arc);

#[cfg(test)]
mod tests {
    use super::*;

    fn escape(s: &str) -> String {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        f.write_str(s);
        buf
    }

    #[test]
    fn no_escaping_needed() {
        assert_eq!(escape("hello world"), "hello world");
    }

    #[test]
    fn escapes_all_special_chars() {
        assert_eq!(
            escape("<div class=\"a\" data-x='b'>&</div>"),
            "&lt;div class=&quot;a&quot; data-x=&#x27;b&#x27;&gt;&amp;&lt;/div&gt;"
        );
    }

    #[test]
    fn only_special_chars() {
        assert_eq!(escape("<>&\"'"), "&lt;&gt;&amp;&quot;&#x27;");
    }

    #[test]
    fn empty_string() {
        assert_eq!(escape(""), "");
    }

    #[test]
    fn multibyte_utf8() {
        assert_eq!(escape("café < résumé"), "café &lt; résumé");
    }

    #[test]
    fn unescaped_passthrough() {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        f.write_str_unescaped("<b>raw</b>");
        assert_eq!(buf, "<b>raw</b>");
    }
}
