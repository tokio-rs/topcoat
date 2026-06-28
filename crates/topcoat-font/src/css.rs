use std::fmt::Write;

/// A [`Write`] adapter that escapes its input as the body of a CSS `<string>`.
///
/// Wrap a destination writer and write the unquoted contents through it; the
/// adapter escapes the characters that are significant inside a CSS string, so
/// the result is safe to place between `"` delimiters. It does not emit the
/// surrounding quotes itself.
pub(crate) struct CssString<'a, W: ?Sized>(pub(crate) &'a mut W);

impl<W: Write + ?Sized> Write for CssString<'_, W> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        s.chars().try_for_each(|c| self.write_char(c))
    }

    fn write_char(&mut self, c: char) -> std::fmt::Result {
        match c {
            '"' => self.0.write_str("\\\""),
            '\\' => self.0.write_str("\\\\"),
            // Control characters are not permitted literally in a CSS string;
            // emit them as a hex escape terminated by a space so a following
            // hex digit is not folded into the escape.
            c if c.is_control() => write!(self.0, "\\{:x} ", c as u32),
            c => self.0.write_char(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn escape(s: &str) -> String {
        let mut out = String::new();
        CssString(&mut out).write_str(s).unwrap();
        out
    }

    #[test]
    fn passes_through_ordinary_text() {
        assert_eq!(escape("Helvetica Neue"), "Helvetica Neue");
    }

    #[test]
    fn passes_through_non_ascii() {
        assert_eq!(escape("Ã¥ Ã¤ Ã¶ æ¼¢å­"), "Ã¥ Ã¤ Ã¶ æ¼¢å­");
    }

    #[test]
    fn escapes_quotes_and_backslashes() {
        assert_eq!(escape("a\"b\\c"), "a\\\"b\\\\c");
    }

    #[test]
    fn escapes_control_characters_with_a_terminating_space() {
        assert_eq!(escape("a\nb"), "a\\a b");
        assert_eq!(escape("\t"), "\\9 ");
    }
}
