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
    pub fn write_str(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let mut start = 0;

        // Offsets of the next byte in each escapable group, or `len` when the
        // group does not occur again. `memchr` searches at most three bytes per
        // call, so the five specials are split across two searches whose results
        // are merged by taking the earlier hit. Both are ASCII, so every hit
        // falls on a UTF-8 character boundary.
        let mut next_amp_lt_gt = memchr::memchr3(b'&', b'<', b'>', bytes).unwrap_or(len);
        let mut next_quote = memchr::memchr2(b'"', b'\'', bytes).unwrap_or(len);

        loop {
            let i = next_amp_lt_gt.min(next_quote);
            if i == len {
                break;
            }

            // Copy the ordinary run preceding the next special in one shot.
            self.buf.push_str(&s[start..i]);

            // Emit the whole run of consecutive specials from the lookup table,
            // so `memchr` is consulted once per run rather than once per byte.
            let mut end = i;
            while let Some(escape) = bytes.get(end).and_then(|&b| ESCAPE_TABLE[b as usize]) {
                self.buf.push_str(escape);
                end += 1;
            }
            start = end;

            // Advance only the cursor(s) that fell inside the run just emitted;
            // a group with no further occurrences is never searched again.
            if next_amp_lt_gt < start {
                next_amp_lt_gt =
                    memchr::memchr3(b'&', b'<', b'>', &bytes[start..]).map_or(len, |o| start + o);
            }
            if next_quote < start {
                next_quote =
                    memchr::memchr2(b'"', b'\'', &bytes[start..]).map_or(len, |o| start + o);
            }
        }

        self.buf.push_str(&s[start..]);
    }

    /// Writes a string without escaping. Use for trusted markup like tags and attributes.
    #[inline]
    pub fn write_str_unescaped(&mut self, s: &str) {
        self.buf.push_str(s);
    }
}

impl std::fmt::Write for Formatter<'_> {
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        Formatter::write_char(self, c);
        Ok(())
    }

    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        Formatter::write_str(self, s);
        Ok(())
    }
}

/// Maps each byte to its HTML escape sequence, or `None` when the byte is safe
/// to emit as-is. Only the five HTML-significant bytes have entries.
const ESCAPE_TABLE: [Option<&'static str>; 256] = {
    let mut table: [Option<&'static str>; 256] = [None; 256];
    table[b'&' as usize] = Some("&amp;");
    table[b'<' as usize] = Some("&lt;");
    table[b'>' as usize] = Some("&gt;");
    table[b'"' as usize] = Some("&quot;");
    table[b'\'' as usize] = Some("&#x27;");
    table
};

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
    fn long_plain_run() {
        // Exercises the bulk copy path over more than a SIMD vector width.
        let s = "abcdefghij".repeat(50);
        assert_eq!(escape(&s), s);
    }

    #[test]
    fn special_inside_long_run() {
        let s = format!("{}<{}", "a".repeat(100), "b".repeat(100));
        let expected = format!("{}&lt;{}", "a".repeat(100), "b".repeat(100));
        assert_eq!(escape(&s), expected);
    }

    #[test]
    fn run_of_consecutive_specials() {
        assert_eq!(escape("abc<>&\"'def"), "abc&lt;&gt;&amp;&quot;&#x27;def");
    }

    #[test]
    fn adjacent_cross_group_specials() {
        // '&' (found by the amp/lt/gt search) immediately followed by '"'
        // (found by the quote search) forces both cursors to advance within a
        // single run and be re-scanned together.
        assert_eq!(escape("x&\"y"), "x&amp;&quot;y");
    }

    #[test]
    fn quotes_only_no_amp_lt_gt() {
        // Only the quote group occurs; the amp/lt/gt group is searched once and
        // never again.
        assert_eq!(escape("'a'b'c'"), "&#x27;a&#x27;b&#x27;c&#x27;");
    }

    #[test]
    fn specials_at_boundaries() {
        assert_eq!(escape("<middle>"), "&lt;middle&gt;");
        assert_eq!(escape("&start"), "&amp;start");
        assert_eq!(escape("end&"), "end&amp;");
    }

    #[test]
    fn multibyte_hugging_specials() {
        // Real specials directly adjacent to multibyte characters exercises the
        // slicing between escapes and multibyte runs.
        assert_eq!(escape("é<é>é&é"), "é&lt;é&gt;é&amp;é");
    }

    #[test]
    fn multibyte_codepoint_embeds_special_byte() {
        // These characters have code points that contain a special byte value
        // (U+263C has 0x3C, U+2026 has 0x26, U+2022 has 0x22, U+2027 has 0x27,
        // U+203E has 0x3E) but never encode to that byte, so they pass through
        // untouched while the interleaved ASCII specials still escape.
        assert_eq!(
            escape("\u{263C}<\u{2026}&\u{2022}\"\u{2027}'\u{203E}>"),
            "\u{263C}&lt;\u{2026}&amp;\u{2022}&quot;\u{2027}&#x27;\u{203E}&gt;"
        );
    }

    #[test]
    fn unescaped_passthrough() {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        f.write_str_unescaped("<b>raw</b>");
        assert_eq!(buf, "<b>raw</b>");
    }
}
