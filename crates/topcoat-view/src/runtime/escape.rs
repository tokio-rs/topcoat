use core::fmt;

use memchr::{memchr2, memchr3};

use crate::runtime::Formatter;

/// The position in an HTML document that a dynamic value is written into.
///
/// Writing through a context makes the value safe for that position. Contexts
/// where HTML provides an escape mechanism rewrite the significant
/// characters; ident contexts, where character references are never decoded,
/// validate instead and panic on characters that could break out of the
/// position:
///
/// | Context          | `&`     | `<`    | `>`    | `"`      | Other        |
/// |------------------|---------|--------|--------|----------|--------------|
/// | `Unescaped`      | -       | -      | -      | -        | -            |
/// | `Text`           | `&amp;` | `&lt;` | `&gt;` | -        | -            |
/// | `AttributeValue` | `&amp;` | -      | -      | `&quot;` | -            |
/// | `Comment`        | `&amp;` | -      | `&gt;` | `&quot;` | -            |
/// | `AttributeKey`   | -       | panic  | panic  | panic    | see below    |
/// | `ElementName`    | -       | panic  | panic  | panic    | see below    |
///
/// The ident contexts reject whitespace, control characters, `"`, `'`, `<`,
/// `>`, `/`, and `=`. This guarantees the identifier cannot terminate or
/// corrupt its token; it does not check full spec validity.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HtmlContext {
    /// Trusted markup written verbatim.
    Unescaped,
    /// A text node between tags. Quotes are not significant here, so the
    /// three escapable characters are found with a single search.
    Text,
    /// A double-quoted attribute value. Only `&` and `"` can terminate or
    /// alter the value, found with a single search.
    AttributeValue,
    /// A machine-readable payload inside an HTML comment, such as the markers
    /// the interactive runtime emits. Escaping `>` guarantees the payload
    /// cannot contain `-->` and terminate the comment, while `&` and `"`
    /// round-trip through entity decoding so double-quoted strings inside the
    /// payload stay unambiguous. Comment data is never entity-decoded by the
    /// browser, so the consumer of the payload must decode it.
    Comment,
    /// An attribute name, validated as an identifier rather than escaped.
    AttributeKey,
    /// A tag name, validated as an identifier rather than escaped.
    ElementName,
}

impl HtmlContext {
    /// Returns a writer that makes everything written to it safe for this
    /// context before appending it to `f`.
    #[inline]
    pub fn writer<'a, 'b>(self, f: &'a mut Formatter<'b>) -> HtmlWriter<'a, 'b> {
        HtmlWriter { context: self, f }
    }

    /// Returns the offset of the next byte that needs escaping in this
    /// context, or `None` when nothing (more) needs escaping.
    #[inline]
    fn find_special(self, haystack: &[u8]) -> Option<usize> {
        match self {
            Self::Text => memchr3(b'&', b'<', b'>', haystack),
            Self::AttributeValue => memchr2(b'&', b'"', haystack),
            Self::Comment => memchr3(b'&', b'>', b'"', haystack),
            Self::Unescaped | Self::AttributeKey | Self::ElementName => None,
        }
    }

    /// Returns the escape sequence for `byte` in this context, or `None`
    /// when it passes through as-is.
    #[inline]
    fn escape(self, byte: u8) -> Option<&'static str> {
        match (self, byte) {
            (Self::Text | Self::AttributeValue | Self::Comment, b'&') => Some("&amp;"),
            (Self::Text, b'<') => Some("&lt;"),
            (Self::Text | Self::Comment, b'>') => Some("&gt;"),
            (Self::AttributeValue | Self::Comment, b'"') => Some("&quot;"),
            _ => None,
        }
    }

    /// Returns `true` if `c` can terminate or corrupt an identifier and is
    /// therefore rejected in the ident contexts.
    #[inline]
    fn forbidden_in_ident(c: char) -> bool {
        c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>' | '/' | '=')
    }

    /// A human-readable description of the context, used in panic messages.
    fn description(self) -> &'static str {
        match self {
            Self::Unescaped => "unescaped content",
            Self::Text => "text",
            Self::AttributeValue => "attribute value",
            Self::Comment => "comment payload",
            Self::AttributeKey => "attribute key",
            Self::ElementName => "element name",
        }
    }
}

/// A writer created by [`HtmlContext::writer`] that makes everything written
/// to it safe for its context before appending it to the underlying
/// [`Formatter`].
///
/// The inherent methods mirror [`fmt::Write`], which the writer also
/// implements for use with `write!`.
pub struct HtmlWriter<'a, 'b> {
    context: HtmlContext,
    f: &'a mut Formatter<'b>,
}

impl HtmlWriter<'_, '_> {
    /// Writes `s`, escaped or validated for this writer's context.
    ///
    /// # Panics
    ///
    /// Panics in the ident contexts ([`AttributeKey`](HtmlContext::AttributeKey),
    /// [`ElementName`](HtmlContext::ElementName)) when `s` contains a
    /// character that could break out of the identifier, since HTML has no
    /// escape mechanism there.
    pub fn write_str(&mut self, s: &str) {
        match self.context {
            HtmlContext::Unescaped => self.f.write_str(s),
            HtmlContext::Text | HtmlContext::AttributeValue | HtmlContext::Comment => {
                self.write_escaped(s);
            }
            HtmlContext::AttributeKey | HtmlContext::ElementName => self.write_ident(s),
        }
    }

    /// Writes a single character, escaped or validated for this writer's
    /// context.
    ///
    /// # Panics
    ///
    /// Panics in the ident contexts when `c` could break out of the
    /// identifier, like [`write_str`](Self::write_str).
    pub fn write_char(&mut self, c: char) {
        match self.context {
            HtmlContext::Unescaped => self.f.write_char(c),
            HtmlContext::Text | HtmlContext::AttributeValue | HtmlContext::Comment => {
                let escape = u8::try_from(c).ok().and_then(|b| self.context.escape(b));
                match escape {
                    Some(escape) => self.f.write_str(escape),
                    None => self.f.write_char(c),
                }
            }
            HtmlContext::AttributeKey | HtmlContext::ElementName => {
                assert!(
                    !HtmlContext::forbidden_in_ident(c),
                    "invalid {}: forbidden character {c:?}",
                    self.context.description(),
                );
                self.f.write_char(c);
            }
        }
    }

    /// Writes `s`, escaping the characters significant in this context.
    fn write_escaped(&mut self, s: &str) {
        let bytes = s.as_bytes();
        let mut start = 0;

        // Escapable bytes are ASCII, so every hit falls on a UTF-8 character
        // boundary.
        while let Some(offset) = self.context.find_special(&bytes[start..]) {
            // Copy the ordinary run preceding the special in one shot.
            let special = start + offset;
            self.f.write_str(&s[start..special]);

            // Emit the whole run of consecutive specials, so the search runs
            // once per run rather than once per byte.
            let mut end = special;
            while let Some(escape) = bytes.get(end).and_then(|&b| self.context.escape(b)) {
                self.f.write_str(escape);
                end += 1;
            }
            start = end;
        }

        self.f.write_str(&s[start..]);
    }

    /// Writes `s` verbatim after checking that every character is allowed in
    /// an ident context, panicking otherwise.
    fn write_ident(&mut self, s: &str) {
        if let Some(c) = s.chars().find(|&c| HtmlContext::forbidden_in_ident(c)) {
            panic!(
                "invalid {} {s:?}: forbidden character {c:?}",
                self.context.description(),
            );
        }
        self.f.write_str(s);
    }
}

impl fmt::Write for HtmlWriter<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        HtmlWriter::write_str(self, s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        HtmlWriter::write_char(self, c);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(context: HtmlContext, s: &str) -> String {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        context.writer(&mut f).write_str(s);
        buf
    }

    fn write_char(context: HtmlContext, c: char) -> String {
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        context.writer(&mut f).write_char(c);
        buf
    }

    #[test]
    fn unescaped_passthrough() {
        assert_eq!(write(HtmlContext::Unescaped, "<b>&\"'</b>"), "<b>&\"'</b>");
        assert_eq!(write_char(HtmlContext::Unescaped, '<'), "<");
    }

    #[test]
    fn text_escapes_amp_lt_gt() {
        assert_eq!(
            write(HtmlContext::Text, "a < b && c > d"),
            "a &lt; b &amp;&amp; c &gt; d"
        );
    }

    #[test]
    fn text_leaves_quotes() {
        assert_eq!(
            write(HtmlContext::Text, "she said \"hi\" and 'bye'"),
            "she said \"hi\" and 'bye'"
        );
    }

    #[test]
    fn text_empty_string() {
        assert_eq!(write(HtmlContext::Text, ""), "");
    }

    #[test]
    fn text_run_of_consecutive_specials() {
        assert_eq!(write(HtmlContext::Text, "abc<>&def"), "abc&lt;&gt;&amp;def");
    }

    #[test]
    fn text_quote_inside_special_run_ends_it() {
        // The '"' has no escape in text, so it terminates the run of escapes
        // and is copied verbatim with the ordinary bytes.
        assert_eq!(write(HtmlContext::Text, "<\">"), "&lt;\"&gt;");
    }

    #[test]
    fn text_specials_at_boundaries() {
        assert_eq!(write(HtmlContext::Text, "<middle>"), "&lt;middle&gt;");
        assert_eq!(write(HtmlContext::Text, "&start"), "&amp;start");
        assert_eq!(write(HtmlContext::Text, "end&"), "end&amp;");
    }

    #[test]
    fn text_long_plain_run() {
        // Exercises the bulk copy path over more than a SIMD vector width.
        let s = "abcdefghij".repeat(50);
        assert_eq!(write(HtmlContext::Text, &s), s);
    }

    #[test]
    fn text_special_inside_long_run() {
        let s = format!("{}<{}", "a".repeat(100), "b".repeat(100));
        let expected = format!("{}&lt;{}", "a".repeat(100), "b".repeat(100));
        assert_eq!(write(HtmlContext::Text, &s), expected);
    }

    #[test]
    fn text_multibyte_utf8() {
        assert_eq!(
            write(HtmlContext::Text, "café < résumé"),
            "café &lt; résumé"
        );
    }

    #[test]
    fn text_multibyte_hugging_specials() {
        // Real specials directly adjacent to multibyte characters exercises
        // the slicing between escapes and multibyte runs.
        assert_eq!(write(HtmlContext::Text, "é<é>é&é"), "é&lt;é&gt;é&amp;é");
    }

    #[test]
    fn text_multibyte_codepoint_embeds_special_byte() {
        // These characters have code points that contain a special byte value
        // (U+263C has 0x3C, U+2026 has 0x26, U+203E has 0x3E) but never
        // encode to that byte, so they pass through untouched while the
        // interleaved ASCII specials still escape.
        assert_eq!(
            write(HtmlContext::Text, "\u{263C}<\u{2026}&\u{203E}>"),
            "\u{263C}&lt;\u{2026}&amp;\u{203E}&gt;"
        );
    }

    #[test]
    fn text_write_char() {
        assert_eq!(write_char(HtmlContext::Text, '<'), "&lt;");
        assert_eq!(write_char(HtmlContext::Text, '"'), "\"");
        assert_eq!(write_char(HtmlContext::Text, 'é'), "é");
    }

    #[test]
    fn attribute_value_escapes_amp_and_quote() {
        assert_eq!(
            write(HtmlContext::AttributeValue, "a=\"b\" & c"),
            "a=&quot;b&quot; &amp; c"
        );
    }

    #[test]
    fn attribute_value_leaves_lt_gt_apostrophe() {
        assert_eq!(
            write(HtmlContext::AttributeValue, "<a href='x'>"),
            "<a href='x'>"
        );
    }

    #[test]
    fn attribute_value_run_of_consecutive_specials() {
        assert_eq!(write(HtmlContext::AttributeValue, "x&\"y"), "x&amp;&quot;y");
    }

    #[test]
    fn attribute_value_write_char() {
        assert_eq!(write_char(HtmlContext::AttributeValue, '"'), "&quot;");
        assert_eq!(write_char(HtmlContext::AttributeValue, '<'), "<");
    }

    #[test]
    fn comment_escapes_amp_gt_quote() {
        assert_eq!(
            write(HtmlContext::Comment, "x --> y && \"z\""),
            "x --&gt; y &amp;&amp; &quot;z&quot;"
        );
    }

    #[test]
    fn comment_leaves_lt_and_apostrophe() {
        assert_eq!(write(HtmlContext::Comment, "<!-- 'a'"), "<!-- 'a'");
    }

    #[test]
    fn comment_cannot_terminate_comment() {
        assert!(!write(HtmlContext::Comment, "--> \"js\" -->").contains("-->"));
    }

    #[test]
    fn attribute_key_accepts_common_names() {
        for key in ["class", "data-x", "aria-label", "@click.prevent", ":href"] {
            assert_eq!(write(HtmlContext::AttributeKey, key), key);
        }
    }

    #[test]
    fn element_name_accepts_custom_elements() {
        assert_eq!(write(HtmlContext::ElementName, "my-element"), "my-element");
    }

    #[test]
    fn ident_contexts_accept_multibyte() {
        // Custom element names may contain characters outside ASCII.
        assert_eq!(write(HtmlContext::ElementName, "emotion-😍"), "emotion-😍");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn attribute_key_rejects_space() {
        write(HtmlContext::AttributeKey, "on click");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn attribute_key_rejects_equals() {
        write(HtmlContext::AttributeKey, "a=b");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn attribute_key_rejects_quote() {
        write(HtmlContext::AttributeKey, "a\"b");
    }

    #[test]
    #[should_panic(expected = "invalid element name")]
    fn element_name_rejects_slash() {
        write(HtmlContext::ElementName, "div/onmouseover");
    }

    #[test]
    #[should_panic(expected = "invalid element name")]
    fn element_name_rejects_gt() {
        write(HtmlContext::ElementName, "div><script");
    }

    #[test]
    #[should_panic(expected = "invalid element name")]
    fn element_name_rejects_control() {
        write(HtmlContext::ElementName, "di\nv");
    }

    #[test]
    #[should_panic(expected = "invalid attribute key")]
    fn ident_write_char_rejects_forbidden() {
        write_char(HtmlContext::AttributeKey, '=');
    }

    #[test]
    fn fmt_write_goes_through_context() {
        use core::fmt::Write;

        let tag = "<tag>";
        let mut buf = String::new();
        let mut f = Formatter::new(&mut buf);
        write!(HtmlContext::Text.writer(&mut f), "a {tag} b").unwrap();
        assert_eq!(buf, "a &lt;tag&gt; b");
    }
}
