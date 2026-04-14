use std::fmt::Write;

pub struct Formatter<'a> {
    write: &'a mut dyn Write,
}

impl<'a> Formatter<'a> {
    #[inline]
    pub fn new(write: &'a mut dyn Write) -> Self {
        Self { write }
    }

    #[inline]
    pub fn write_char(&mut self, c: char) -> std::fmt::Result {
        match c {
            '&' => self.write.write_str("&amp;"),
            '<' => self.write.write_str("&lt;"),
            '>' => self.write.write_str("&gt;"),
            '"' => self.write.write_str("&quot;"),
            '\'' => self.write.write_str("&#x27;"),
            _ => self.write_char_unescaped(c),
        }
    }

    #[inline]
    pub fn write_char_unescaped(&mut self, c: char) -> std::fmt::Result {
        self.write.write_char(c)
    }

    #[inline]
    pub fn write_str(&mut self, s: &str) -> std::fmt::Result {
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
                self.write.write_str(&s[last..i])?;
            }
            self.write.write_str(escape)?;
            last = i + 1;
        }

        if last < s.len() {
            self.write.write_str(&s[last..])?;
        }

        Ok(())
    }

    #[inline]
    pub fn write_str_unescaped(&mut self, s: &str) -> std::fmt::Result {
        self.write.write_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escape(s: &str) -> String {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        f.write_str(s).unwrap();
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
