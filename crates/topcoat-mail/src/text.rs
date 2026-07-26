//! Deriving the plain-text body from rendered HTML.

/// Derives a plain-text rendering from HTML the view renderer produced.
///
/// The converter leans on the renderer's guarantees (balanced tags, quoted
/// attributes, escaped text), so it scans tags instead of parsing arbitrary
/// HTML; raw markup spliced into a view converts on a best-effort basis.
/// Content flows as lines: block elements start a new line, paragraphs and
/// headings stand as separate blocks, list items get a marker, links keep
/// their target in parentheses, and images fall back to their alt text.
/// The head of a document, styles, and scripts contribute nothing.
pub(crate) fn text_from_html(html: &str) -> String {
    let mut converter = Converter::default();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        let (text, tag) = rest.split_at(start);
        converter.text(text);
        rest = converter.tag(tag);
    }
    converter.text(rest);
    converter.finish()
}

#[derive(Default)]
struct Converter {
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

impl Converter {
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

    /// Handles the tag at the start of `s` and returns what follows it.
    fn tag<'a>(&mut self, s: &'a str) -> &'a str {
        if let Some(comment) = s.strip_prefix("<!--") {
            return comment.find("-->").map_or("", |end| &comment[end + 3..]);
        }
        if s.starts_with("<!") || s.starts_with("<?") {
            return s.find('>').map_or("", |end| &s[end + 1..]);
        }

        let Some(end) = tag_end(s) else { return "" };
        let (tag, rest) = (&s[1..end], &s[end + 1..]);
        let (closing, tag) = match tag.strip_prefix('/') {
            Some(tag) => (true, tag),
            None => (false, tag),
        };
        let name_end = tag
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(tag.len());
        let (name, attributes) = tag.split_at(name_end);
        let name = name.to_ascii_lowercase();

        if closing {
            self.close(&name);
        } else {
            self.open(&name, attributes);
        }
        rest
    }

    /// Handles an opening tag.
    fn open(&mut self, name: &str, attributes: &str) {
        match name {
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
                let target = attribute(attributes, "href").unwrap_or_default();
                self.links
                    .push((self.out.len(), decode(&target).into_owned()));
            }
            "img" => {
                if let Some(alt) = attribute(attributes, "alt") {
                    self.text(&alt);
                }
            }
            _ => self.separate(name),
        }
    }

    /// Handles a closing tag.
    fn close(&mut self, name: &str) {
        match name {
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
            _ => self.separate(name),
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

/// Reads an attribute value from a tag's attribute list.
fn attribute(attributes: &str, name: &str) -> Option<String> {
    let mut rest = attributes;
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
            return Some(value.to_owned());
        }
        rest = after;
    }
}

/// Decodes the character entities the HTML escaper produces.
fn decode(raw: &str) -> std::borrow::Cow<'_, str> {
    if !raw.contains('&') {
        return std::borrow::Cow::Borrowed(raw);
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
    std::borrow::Cow::Owned(out)
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
        assert_eq!(text_from_html(html), "Title\n\nOne\n\na\nb\nc");
    }

    #[test]
    fn whitespace_collapses() {
        let html = "<p>Spread\n    out\t text </p>";
        assert_eq!(text_from_html(html), "Spread out text");
    }

    #[test]
    fn inline_elements_flow() {
        let html = "<p><strong>Bold</strong> and <em>italic</em>.</p>";
        assert_eq!(text_from_html(html), "Bold and italic.");
    }

    #[test]
    fn lists_get_markers() {
        let bullets = "<ul><li>One</li><li>Two</li></ul>";
        assert_eq!(text_from_html(bullets), "- One\n- Two");

        let numbered = "<ol><li>First</li><li>Second</li></ol>";
        assert_eq!(text_from_html(numbered), "1. First\n2. Second");
    }

    #[test]
    fn links_keep_their_target() {
        let labeled = r#"<a href="https://example.com/?a=1&amp;b=2">Click here</a>"#;
        assert_eq!(
            text_from_html(labeled),
            "Click here (https://example.com/?a=1&b=2)"
        );

        let bare = r#"<a href="https://example.com">https://example.com</a>"#;
        assert_eq!(text_from_html(bare), "https://example.com");

        let mailto = r#"<a href="mailto:ada@example.com">Write Ada</a>"#;
        assert_eq!(text_from_html(mailto), "Write Ada");
    }

    #[test]
    fn images_fall_back_to_alt_text() {
        let html = r#"<p><img src="cid:logo" alt="The logo"> and <img src="cid:pixel"></p>"#;
        assert_eq!(text_from_html(html), "The logo and");
    }

    #[test]
    fn head_styles_and_scripts_contribute_nothing() {
        let html = "<!DOCTYPE html><html><head><title>Page</title>\
                    <style>body { color: red; }</style></head>\
                    <body><script>let x = 1;</script><p>Visible</p></body></html>";
        assert_eq!(text_from_html(html), "Visible");
    }

    #[test]
    fn layout_tables_flatten_into_lines() {
        let html = "<table><tr><td>A</td><td>B</td></tr><tr><td>C</td></tr></table>";
        assert_eq!(text_from_html(html), "A\nB\nC");
    }

    #[test]
    fn entities_decode() {
        let html = "<p>&lt;3 &amp;&nbsp;m&#111;re&#x21;</p>";
        assert_eq!(text_from_html(html), "<3 & more!");
    }

    #[test]
    fn malformed_markup_degrades_gracefully() {
        assert_eq!(text_from_html("a < b"), "a");
        assert_eq!(text_from_html("<p>unclosed"), "unclosed");
        assert_eq!(text_from_html("<!-- open comment"), "");
        assert_eq!(text_from_html(""), "");
    }
}
