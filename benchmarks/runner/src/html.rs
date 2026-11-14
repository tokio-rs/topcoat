//! HTML inspection shared by `verify` and the client payload measurement.

use std::collections::BTreeMap;

use ego_tree::iter::Edge;
use scraper::{Html, Node, Selector};

/// Elements counted by the parity check.
pub const COUNTED_ELEMENTS: [&str; 9] =
    ["a", "article", "tr", "td", "li", "button", "h1", "h2", "h3"];

/// Elements whose content is not visible text.
const HIDDEN: [&str; 5] = ["script", "style", "template", "noscript", "svg"];

/// Elements that start a new line of text.
const BLOCKS: [&str; 41] = [
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "br",
    "dd",
    "details",
    "dialog",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "li",
    "main",
    "nav",
    "ol",
    "p",
    "pre",
    "section",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
];

/// A parsed HTML document.
pub struct Page {
    document: Html,
}

/// A URL a page references, with the category its tag implies.
pub struct Reference {
    pub url: String,
    pub category: Option<&'static str>,
}

impl Page {
    pub fn parse(html: &str) -> Self {
        Self {
            document: Html::parse_document(html),
        }
    }

    /// Visible text, one entry per block of text, with whitespace collapsed. Comments,
    /// scripts, styles, templates and inline SVG are ignored.
    pub fn text_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();
        let mut hidden_depth = 0usize;
        let flush = |current: &mut String, lines: &mut Vec<String>| {
            let line = current.split_whitespace().collect::<Vec<_>>().join(" ");
            if !line.is_empty() {
                lines.push(line);
            }
            current.clear();
        };
        for edge in self.document.tree.root().traverse() {
            match edge {
                Edge::Open(node) => match node.value() {
                    Node::Element(element) => {
                        let name = element.name();
                        if HIDDEN.contains(&name) {
                            hidden_depth += 1;
                        } else if hidden_depth == 0 && BLOCKS.contains(&name) {
                            flush(&mut current, &mut lines);
                        }
                    }
                    Node::Text(text) if hidden_depth == 0 => {
                        current.push_str(text);
                    }
                    _ => {}
                },
                Edge::Close(node) => {
                    if let Node::Element(element) = node.value() {
                        let name = element.name();
                        if HIDDEN.contains(&name) {
                            hidden_depth -= 1;
                        } else if hidden_depth == 0 && BLOCKS.contains(&name) {
                            flush(&mut current, &mut lines);
                        }
                    }
                }
            }
        }
        flush(&mut current, &mut lines);
        lines
    }

    /// How often each of [`COUNTED_ELEMENTS`] appears.
    pub fn element_counts(&self) -> BTreeMap<String, usize> {
        let mut counts: BTreeMap<String, usize> = COUNTED_ELEMENTS
            .iter()
            .map(|name| ((*name).to_owned(), 0))
            .collect();
        for node in self.document.tree.root().descendants() {
            if let Node::Element(element) = node.value()
                && let Some(count) = counts.get_mut(element.name())
            {
                *count += 1;
            }
        }
        counts
    }

    /// Scripts, stylesheets and preloads the browser downloads.
    /// Scripts with `nomodule` are left out: browsers that support modules never fetch them.
    pub fn references(&self) -> Vec<Reference> {
        let mut references = Vec::new();
        for element in self
            .document
            .select(&selector("script[src]:not([nomodule])"))
        {
            if let Some(src) = element.value().attr("src") {
                references.push(Reference {
                    url: src.to_owned(),
                    category: Some("js"),
                });
            }
        }
        for element in self.document.select(&selector("link[href]")) {
            let attrs = element.value();
            let rel = attrs.attr("rel").unwrap_or("").to_ascii_lowercase();
            let rels: Vec<&str> = rel.split_whitespace().collect();
            let category = if rels.contains(&"stylesheet") {
                Some("css")
            } else if rels.contains(&"modulepreload") {
                Some("js")
            } else if rels.contains(&"preload") {
                match attrs.attr("as").unwrap_or("") {
                    "script" => Some("js"),
                    "style" => Some("css"),
                    _ => None,
                }
            } else {
                continue;
            };
            if let Some(href) = attrs.attr("href") {
                references.push(Reference {
                    url: href.to_owned(),
                    category,
                });
            }
        }
        references
    }

    /// The content of inline `<script>` elements.
    pub fn inline_scripts(&self) -> Vec<String> {
        self.document
            .select(&selector("script:not([src])"))
            .map(|element| element.text().collect())
            .collect()
    }

    /// Total bytes of inline `<style>` content.
    pub fn inline_style_bytes(&self) -> usize {
        self.document
            .select(&selector("style"))
            .map(|element| element.text().map(str::len).sum::<usize>())
            .sum()
    }
}

fn selector(text: &str) -> Selector {
    Selector::parse(text).expect("valid selector")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_ignores_markup_differences_that_do_not_change_content() {
        let compact = Page::parse(
            "<html><head><title>T</title></head><body><div><span>$1,234.50</span></div><p>4.0 &amp; up</p></body></html>",
        );
        let verbose = Page::parse(
            "<html><head><title>T</title><script>var x = 1;</script></head><body>\n  <div>\n    <span>$1,<!---->234.50</span>\n  </div>\n  <p>4.0 &#38; up</p><style>p{}</style></body></html>",
        );
        assert_eq!(compact.text_lines(), verbose.text_lines());
    }

    #[test]
    fn text_detects_changed_content() {
        let a = Page::parse("<body><p>Sold out</p></body>");
        let b = Page::parse("<body><p>In stock</p></body>");
        assert_ne!(a.text_lines(), b.text_lines());
    }

    #[test]
    fn references_skip_scripts_modern_browsers_ignore() {
        let page = Page::parse(
            r#"<head><script nomodule src="/polyfills.js"></script><script src="/app.js" type="module"></script></head>"#,
        );
        let urls: Vec<_> = page.references().into_iter().map(|r| r.url).collect();
        assert_eq!(urls, ["/app.js"]);
    }

    #[test]
    fn references_cover_scripts_styles_and_preloads() {
        let page = Page::parse(
            r#"<head><link rel="stylesheet" href="/a.css"><link rel="modulepreload" href="/b.js"><link rel="icon" href="/i.png"></head><body><script src="/c.js"></script><script>inline()</script></body>"#,
        );
        let urls: Vec<_> = page.references().into_iter().map(|r| r.url).collect();
        assert_eq!(urls.len(), 3);
        for url in ["/a.css", "/b.js", "/c.js"] {
            assert!(urls.iter().any(|u| u == url), "{url} missing");
        }
        assert_eq!(page.inline_scripts().len(), 1);
    }
}
