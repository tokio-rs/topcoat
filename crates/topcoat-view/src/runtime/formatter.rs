use std::fmt::Write;

/// An HTML-aware writer that escapes text content by default.
///
/// `Formatter` wraps a [`String`] buffer and provides paired
/// escaped/unescaped methods for writing strings and characters. The escaped
/// variants handle the five characters that are meaningful in HTML:
///
/// | Character | Escaped as |
/// |-----------|------------|
/// | `&`       | `&amp;`    |
/// | `<`       | `&lt;`     |
/// | `>`       | `&gt;`     |
/// | `"`       | `&quot;`   |
/// | `'`       | `&#x27;`   |
///
/// Use the escaped methods ([`write_str`](Self::write_str),
/// [`write_char`](Self::write_char)) for user-provided or dynamic content, and
/// the unescaped methods ([`write_str_unescaped`](Self::write_str_unescaped),
/// [`write_char_unescaped`](Self::write_char_unescaped)) for trusted markup
/// like tags and attributes.
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
        };
    }

    /// Writes a single character without escaping. Use for trusted content only.
    #[inline]
    pub fn write_char_unescaped(&mut self, c: char) {
        self.buf.push(c);
    }

    /// Writes a string, escaping any HTML-significant characters.
    ///
    /// Scans for safe spans and flushes them in bulk, only falling back to
    /// per-entity writes when a special character is encountered. This avoids
    /// the overhead of writing one character at a time for strings that are
    /// mostly (or entirely) safe.
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
        self.buf.write_str(s);
    }
}

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
        f.write_str_unescaped("<b>raw</b>").unwrap();
        assert_eq!(buf, "<b>raw</b>");
    }
}
