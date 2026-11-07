//! Deriving the plain-text body from rendered HTML.

use std::borrow::Cow;

/// A plain-text rendering of HTML the view renderer produced.
///
/// Expects balanced tags, quoted attributes, and escaped text from the view
/// renderer. Raw markup is converted on a best-effort basis.
///
/// Preserves block boundaries, list markers, link targets, and image alt text.
/// Omits document head content, styles, and scripts.
#[derive(Default)]
pub(crate) struct PlainText {
    out: String,
    /// Line breaks owed before the next content: one within a block, two
    /// between blocks.
    breaks: u8,
    /// Whether a space is owed before the next content.
    space: bool,
    /// Depth of open elements whose content is dropped entirely.
    hidden: usize,
    /// Open links: where each label starts in `out`, and the target.
    links: Vec<(usize, String)>,
    /// Open lists: the last item number, or `None` in a bullet list.
    lists: Vec<Option<u32>>,
}

impl PlainText {
    /// Converts `html` to plain text.
    pub(crate) fn from_html(html: &str) -> String {
        let mut text = PlainText::default();
        for token in Tokens::new(html) {
            match token {
                Token::Text(raw) => text.text(raw),
                Token::Open(tag) => text.open(&tag),
                Token::Close(tag) => text.close(&tag),
            }
        }
        text.finish()
    }

    /// Emits a text run, collapsing whitespace and decoding entities.
    fn text(&mut self, raw: &str) {
        if self.hidden > 0 {
            return;
        }
        for c in decode(raw).chars() {
            if c.is_whitespace() {
                self.space = true;
            } else {
                self.flush();
                self.out.push(c);
            }
        }
    }

    /// Handles an opening tag.
    fn open(&mut self, tag: &Tag<'_>) {
        match &*tag.name {
            "head" | "style" | "script" | "template" | "svg" | "title" => self.hidden += 1,
            _ if self.hidden > 0 => {}
            "br" => self.break_line(1),
            "ul" => {
                self.break_line(2);
                self.lists.push(None);
            }
            "ol" => {
                self.break_line(2);
                self.lists.push(Some(0));
            }
            "li" => {
                self.break_line(1);
                let marker = match self.lists.last_mut() {
                    Some(Some(number)) => {
                        *number += 1;
                        format!("{number}. ")
                    }
                    _ => "- ".to_owned(),
                };
                self.flush();
                self.out.push_str(&marker);
                self.space = false;
            }
            "a" => {
                let target = tag.attribute("href").map(decode).unwrap_or_default();
                self.links.push((self.out.len(), target.into_owned()));
            }
            "img" => {
                if let Some(alt) = tag.attribute("alt") {
                    self.text(alt);
                }
            }
            name => self.separate(name),
        }
    }

    /// Handles a closing tag.
    fn close(&mut self, tag: &Tag<'_>) {
        match &*tag.name {
            "head" | "style" | "script" | "template" | "svg" | "title" => {
                self.hidden = self.hidden.saturating_sub(1);
            }
            _ if self.hidden > 0 => {}
            "ul" | "ol" => {
                self.lists.pop();
                self.break_line(2);
            }
            "li" => self.break_line(1),
            "a" => self.close_link(),
            name => self.separate(name),
        }
    }

    /// Applies an element's separation: blocks stand apart, lines break,
    /// and inline elements flow.
    fn separate(&mut self, name: &str) {
        match name {
            "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "pre" | "table"
            | "figure" | "address" | "hr" => self.break_line(2),
            "div" | "section" | "article" | "header" | "footer" | "main" | "nav" | "aside"
            | "form" | "fieldset" | "tr" | "td" | "th" | "caption" | "figcaption" | "dt" | "dd"
            | "details" | "summary" => self.break_line(1),
            _ => {}
        }
    }

    /// Finishes an `<a>` element, appending its target after the label.
    fn close_link(&mut self) {
        let Some((start, target)) = self.links.pop() else {
            return;
        };
        let served = target.starts_with("https://") || target.starts_with("http://");
        if served && self.out[start..].trim() != target {
            self.out.push_str(" (");
            self.out.push_str(&target);
            self.out.push(')');
        }
    }

    /// Owes the next content at least `count` line breaks.
    fn break_line(&mut self, count: u8) {
        if !self.out.is_empty() {
            self.breaks = self.breaks.max(count);
        }
    }

    /// Emits the owed line breaks or space before new content.
    fn flush(&mut self) {
        if self.breaks > 0 {
            for _ in 0..self.breaks {
                self.out.push('\n');
            }
        } else if self.space && !self.out.is_empty() && !self.out.ends_with(char::is_whitespace) {
            self.out.push(' ');
        }
        self.breaks = 0;
        self.space = false;
    }

    /// Returns the converted text without trailing whitespace.
    fn finish(mut self) -> String {
        self.out.truncate(self.out.trim_end().len());
        self.out
    }
}

/// The text runs and tags of an HTML string, in document order.
///
/// Skips comments, doctypes, and processing instructions. Drops the rest of
/// the input at an unterminated tag.
struct Tokens<'a> {
    rest: &'a str,
}

impl<'a> Tokens<'a> {
    fn new(html: &'a str) -> Tokens<'a> {
        Tokens { rest: html }
    }

    /// Consumes the markup at the start of `rest`, returning it if it is a
    /// tag.
    fn markup(&mut self) -> Option<Token<'a>> {
        let rest = self.rest;
        if let Some(comment) = rest.strip_prefix("<!--") {
            self.rest = comment.find("-->").map_or("", |end| &comment[end + 3..]);
            return None;
        }
        if rest.starts_with("<!") || rest.starts_with("<?") {
            self.rest = rest.find('>').map_or("", |end| &rest[end + 1..]);
            return None;
        }

        let Some(end) = tag_end(rest) else {
            self.rest = "";
            return None;
        };
        self.rest = &rest[end + 1..];
        match rest[1..end].strip_prefix('/') {
            Some(inner) => Some(Token::Close(Tag::parse(inner))),
            None => Some(Token::Open(Tag::parse(&rest[1..end]))),
        }
    }
}

impl<'a> Iterator for Tokens<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        while !self.rest.is_empty() {
            match self.rest.find('<') {
                None => return Some(Token::Text(std::mem::take(&mut self.rest))),
                Some(0) => {
                    if let Some(token) = self.markup() {
                        return Some(token);
                    }
                }
                Some(start) => {
                    let (text, rest) = self.rest.split_at(start);
                    self.rest = rest;
                    return Some(Token::Text(text));
                }
            }
        }
        None
    }
}

/// A piece of HTML produced by [`Tokens`].
enum Token<'a> {
    /// Text between tags, with entities still encoded.
    Text(&'a str),
    /// An opening or self-closing tag.
    Open(Tag<'a>),
    /// A closing tag.
    Close(Tag<'a>),
}

/// A tag's name and attributes.
struct Tag<'a> {
    /// The lowercase element name.
    name: Cow<'a, str>,
    /// The raw attribute list following the name.
    attributes: &'a str,
}

impl<'a> Tag<'a> {
    /// Parses the inside of a tag, without its brackets or closing slash.
    fn parse(inner: &'a str) -> Tag<'a> {
        let name_end = inner
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(inner.len());
        let (name, attributes) = inner.split_at(name_end);
        let name = if name.bytes().any(|b| b.is_ascii_uppercase()) {
            Cow::Owned(name.to_ascii_lowercase())
        } else {
            Cow::Borrowed(name)
        };
        Tag { name, attributes }
    }

    /// The raw value of the attribute `name`, with entities still encoded.
    fn attribute(&self, name: &str) -> Option<&'a str> {
        let mut rest = self.attributes;
        loop {
            rest = rest.trim_start();
            if rest.is_empty() || rest == "/" {
                return None;
            }

            let name_end = rest
                .find(|c: char| c.is_whitespace() || c == '=')
                .unwrap_or(rest.len());
            let (candidate, after) = rest.split_at(name_end);
            let Some(after) = after.trim_start().strip_prefix('=') else {
                // An attribute without a value.
                rest = after;
                continue;
            };

            let after = after.trim_start();
            let (value, after) = if let Some(open @ ('"' | '\'')) = after.chars().next() {
                let value = &after[1..];
                match value.find(open) {
                    Some(end) => (&value[..end], &value[end + 1..]),
                    None => (value, ""),
                }
            } else {
                let end = after
                    .find(|c: char| c.is_whitespace() || c == '>')
                    .unwrap_or(after.len());
                after.split_at(end)
            };

            if candidate.eq_ignore_ascii_case(name) {
                return Some(value);
            }
            rest = after;
        }
    }
}

/// Finds the `>` closing the tag at the start of `s`, skipping quoted
/// attribute values.
fn tag_end(s: &str) -> Option<usize> {
    let mut quote = None;
    for (index, c) in s.char_indices() {
        match (quote, c) {
            (None, '>') => return Some(index),
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if c == open => quote = None,
            _ => {}
        }
    }
    None
}

/// Decodes the character entities the HTML escaper produces.
fn decode(raw: &str) -> Cow<'_, str> {
    if !raw.contains('&') {
        return Cow::Borrowed(raw);
    }

    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];

        let entity = rest
            .find(';')
            .filter(|&end| end <= 32)
            .and_then(|end| decode_entity(&rest[1..end]).map(|c| (c, end)));
        if let Some((c, end)) = entity {
            out.push(c);
            rest = &rest[end + 1..];
        } else {
            out.push('&');
            rest = &rest[1..];
        }
    }
    out.push_str(rest);
    Cow::Owned(out)
}

/// Decodes a single entity name or number, without its `&` and `;`.
fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some(' '),
        _ => {
            let code = match entity
                .strip_prefix("#x")
                .or_else(|| entity.strip_prefix("#X"))
            {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => entity.strip_prefix('#')?.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_stand_apart_and_lines_break() {
        let html = "<h1>Title</h1><p>One</p><div>a</div><div>b<br>c</div>";
        assert_eq!(PlainText::from_html(html), "Title\n\nOne\n\na\nb\nc");
    }

    #[test]
    fn whitespace_collapses() {
        let html = "<p>Spread\n    out\t text </p>";
        assert_eq!(PlainText::from_html(html), "Spread out text");
    }

    #[test]
    fn inline_elements_flow() {
        let html = "<p><strong>Bold</strong> and <em>italic</em>.</p>";
        assert_eq!(PlainText::from_html(html), "Bold and italic.");
    }

    #[test]
    fn lists_get_markers() {
        let bullets = "<ul><li>One</li><li>Two</li></ul>";
        assert_eq!(PlainText::from_html(bullets), "- One\n- Two");

        let numbered = "<ol><li>First</li><li>Second</li></ol>";
        assert_eq!(PlainText::from_html(numbered), "1. First\n2. Second");
    }

    #[test]
    fn links_keep_their_target() {
        let labeled = r#"<a href="https://example.com/?a=1&amp;b=2">Click here</a>"#;
        assert_eq!(
            PlainText::from_html(labeled),
            "Click here (https://example.com/?a=1&b=2)"
        );

        let bare = r#"<a href="https://example.com">https://example.com</a>"#;
        assert_eq!(PlainText::from_html(bare), "https://example.com");

        let mailto = r#"<a href="mailto:ada@example.com">Write Ada</a>"#;
        assert_eq!(PlainText::from_html(mailto), "Write Ada");
    }

    #[test]
    fn images_fall_back_to_alt_text() {
        let html = r#"<p><img src="cid:logo" alt="The logo"> and <img src="cid:pixel"></p>"#;
        assert_eq!(PlainText::from_html(html), "The logo and");
    }

    #[test]
    fn head_styles_and_scripts_contribute_nothing() {
        let html = "<!DOCTYPE html><html><head><title>Page</title>\
                    <style>body { color: red; }</style></head>\
                    <body><script>let x = 1;</script><p>Visible</p></body></html>";
        assert_eq!(PlainText::from_html(html), "Visible");
    }

    #[test]
    fn layout_tables_flatten_into_lines() {
        let html = "<table><tr><td>A</td><td>B</td></tr><tr><td>C</td></tr></table>";
        assert_eq!(PlainText::from_html(html), "A\nB\nC");
    }

    #[test]
    fn entities_decode() {
        let html = "<p>&lt;3 &amp;&nbsp;m&#111;re&#x21;</p>";
        assert_eq!(PlainText::from_html(html), "<3 & more!");
    }

    #[test]
    fn tag_names_ignore_case() {
        let html = "<P>One</P><UL><LI>Two</LI></UL>";
        assert_eq!(PlainText::from_html(html), "One\n\n- Two");
    }

    #[test]
    fn malformed_markup_degrades_gracefully() {
        assert_eq!(PlainText::from_html("a < b"), "a");
        assert_eq!(PlainText::from_html("<p>unclosed"), "unclosed");
        assert_eq!(PlainText::from_html("<!-- open comment"), "");
        assert_eq!(PlainText::from_html(""), "");
    }
}
