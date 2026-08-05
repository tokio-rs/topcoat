//! Helper functions for constructing view! AST elements and attributes.

use heck::ToKebabCase;
use proc_macro2::Span;
use syn::{Ident, LitStr, parse_quote};
use topcoat_view_grammar::{
    attributes::{Attribute, AttributeKey, AttributeNode, AttributeValue, Attributes},
    view::{
        ClosingTag, Element, ElementName, HtmlIdent, HtmlIdentPart, HtmlIdentSegment,
        HtmlIdentSeparator, Node, Nodes, OpeningTag, SelfClosingTag,
    },
};

// ---------------------------------------------------------------------------
// Helper functions for constructing elements and attributes
// ---------------------------------------------------------------------------

/// Constructs a `Node::Text` from a string.
pub(crate) fn text_node(content: &str) -> Node {
    Node::Text(LitStr::new(content, Span::call_site()))
}

/// Creates an `Ident` that can be a Rust keyword (e.g., "type", "for").
/// `syn::parse_str::<Ident>` uses `Ident::parse`, which rejects keywords.
/// The fallback uses `Ident::new` directly for keyword-safe identifiers.
///
/// Panics if `name` is empty or starts with a digit, because those inputs
/// produce unhelpful panics inside `Ident::new`.
pub(crate) fn make_ident(name: &str) -> Ident {
    assert!(
        !name.is_empty(),
        "make_ident: identifier cannot be empty \
        (source: attribute or tag name)"
    );
    syn::parse_str(name).unwrap_or_else(|_| Ident::new(name, Span::call_site()))
}

/// Constructs an `ElementName` from a tag name string.
pub(crate) fn make_element_name(tag: &str) -> ElementName {
    ElementName::Ident(HtmlIdent {
        first: make_ident(tag),
        rest: vec![],
    })
}

/// Constructs a normal HTML element with opening and closing tags, wrapped in Node.
pub(crate) fn html_element(tag: &str, children: Nodes) -> Node {
    let attributes = Attributes::default();
    Node::Element(Box::new(normal_element_with_attrs(
        tag, attributes, children,
    )))
}

/// Constructs a normal HTML element with custom attributes.
pub(crate) fn normal_element_with_attrs(
    tag: &str,
    attributes: Attributes,
    children: Nodes,
) -> Element {
    let closing_name = make_element_name(tag);
    let opening = OpeningTag {
        lt: parse_quote!(<),
        name: make_element_name(tag),
        attributes,
        gt: parse_quote!(>),
    };
    let closing = ClosingTag {
        lt: parse_quote!(<),
        slash: parse_quote!(/),
        name: closing_name,
        gt: parse_quote!(>),
    };
    Element::Normal {
        opening_tag: opening,
        children,
        closing_tag: closing,
    }
}

/// Constructs a void HTML element (no closing tag, no children).
pub(crate) fn void_element(tag: &str) -> Element {
    void_element_with_attrs(tag, Attributes::default())
}

/// Constructs a void HTML element with custom attributes.
pub(crate) fn void_element_with_attrs(tag: &str, attributes: Attributes) -> Element {
    Element::Void {
        tag: OpeningTag {
            lt: parse_quote!(<),
            name: make_element_name(tag),
            attributes,
            gt: parse_quote!(>),
        },
    }
}

/// Constructs a self-closing element (`<tag ... />`).
pub(crate) fn self_closing_element(tag: &str, attributes: Attributes) -> Element {
    Element::SelfClosing {
        tag: SelfClosingTag {
            lt: parse_quote!(<),
            name: make_element_name(tag),
            attributes,
            slash: parse_quote!(/),
            gt: parse_quote!(>),
        },
    }
}

/// Creates a key=value attribute where the key may contain hyphens
/// (e.g. `data-lang`, `data-title`). Splits on `-` and builds an
/// `HtmlIdent` with `first` + `rest` segments.
pub(crate) fn create_attribute_data(key: &str, value: &str) -> Attribute {
    let segments: Vec<&str> = key.splitn(2, '-').collect();
    let rest: Vec<HtmlIdentSegment> = if segments.len() == 2 && !segments[1].is_empty() {
        let dash: syn::token::Minus = parse_quote!(-);
        vec![HtmlIdentSegment {
            separator: HtmlIdentSeparator::Dash(dash),
            part: HtmlIdentPart::Ident(Ident::new(segments[1], Span::call_site())),
        }]
    } else {
        Vec::new()
    };
    Attribute {
        key: AttributeKey::Ident(HtmlIdent {
            first: make_ident(segments[0]),
            rest,
        }),
        eq: parse_quote!(=),
        value: AttributeValue::LitStr(LitStr::new(value, Span::call_site())),
    }
}

/// Creates a key=value attribute.
pub(crate) fn create_attribute(key: &str, value: &str) -> Attribute {
    Attribute {
        key: AttributeKey::Ident(HtmlIdent {
            first: make_ident(key),
            rest: vec![],
        }),
        eq: parse_quote!(=),
        value: AttributeValue::LitStr(LitStr::new(value, Span::call_site())),
    }
}

/// Creates a boolean attribute (key with empty value, e.g., `checked=""`).
pub(crate) fn create_attribute_bool(key: &str) -> Attribute {
    Attribute {
        key: AttributeKey::Ident(HtmlIdent {
            first: make_ident(key),
            rest: vec![],
        }),
        eq: parse_quote!(=),
        value: AttributeValue::LitStr(LitStr::new("", Span::call_site())),
    }
}

/// Wraps a vec of `Attribute`s into an `Attributes` value.
pub(crate) fn with_attributes(attrs: Vec<Attribute>) -> Attributes {
    Attributes {
        cx: None,
        items: attrs.into_iter().map(AttributeNode::Attribute).collect(),
    }
}

/// Generates a kebab-case slug from heading text for use as an HTML `id` attribute.
///
/// Converts the text to lowercase kebab-case using `heck`, collapsing consecutive
/// dashes and trimming leading/trailing dashes. Empty or punctuation-only input
/// produces an empty string.
pub(crate) fn slugify(text: &str) -> String {
    let slug = text.to_kebab_case().replace("--", "-");
    let trimmed = slug.trim_matches('-');
    trimmed.to_string()
}

// ---------------------------------------------------------------------------
// Code block metadata
// ---------------------------------------------------------------------------

/// Parsed metadata from a fenced code block's info string.
///
/// `lang` comes from `Code.lang` (the first token). The remaining fields
/// come from `Code.meta` (everything after the language identifier):
/// `{1,3}` line ranges, `title="..."`, and `/term/` emphasis.
#[derive(Debug, Default, Clone)]
pub(crate) struct CodeMeta {
    /// The language identifier (first token of the fence info string).
    pub lang: Option<String>,
    /// Line highlight ranges (e.g. `{1,3}` or `{1-5}`).
    pub lines: Option<String>,
    /// Title string (e.g. `title="file.rs"`).
    pub title: Option<String>,
    /// Term emphasis patterns (e.g. `/keyword/`).
    pub emphasis: Vec<String>,
}

/// Parses a fenced code block into a `CodeMeta` struct.
///
/// Reads `code.lang` for the language and tokenizes `code.meta` for
/// line ranges, title, and term emphasis.
pub(crate) fn parse_code_meta(code: &markdown::mdast::Code) -> CodeMeta {
    let lang = code.lang.clone();
    let mut lines: Vec<String> = Vec::new();
    let mut title: Option<String> = None;
    let mut emphasis: Vec<String> = Vec::new();

    if let Some(meta) = &code.meta {
        for token in meta.split_whitespace() {
            if token.starts_with('{') && token.ends_with('}') {
                lines.push(token[1..token.len() - 1].to_string());
            } else if let Some(t) = token
                .strip_prefix("title=\"")
                .and_then(|s| s.strip_suffix('"'))
                .or(token
                    .strip_prefix("title='")
                    .and_then(|s| s.strip_suffix('\'')))
            {
                title = Some(t.to_string());
            } else if token.starts_with('/') && token.ends_with('/') && token.len() > 1 {
                emphasis.push(token[1..token.len() - 1].to_string());
            }
        }
    }

    CodeMeta {
        lang,
        lines: if lines.is_empty() {
            None
        } else {
            Some(lines.join(","))
        },
        title,
        emphasis,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- slugify tests ----

    #[test]
    fn slugify_simple_word() {
        assert_eq!(slugify("Hello"), "hello");
    }

    #[test]
    fn slugify_multiple_words() {
        assert_eq!(
            slugify("Setup and Configuration"),
            "setup-and-configuration"
        );
    }

    #[test]
    fn slugify_strips_punctuation() {
        assert_eq!(slugify("Hello, World!"), "hello-world");
    }

    #[test]
    fn slugify_with_numbers() {
        assert_eq!(slugify("Step 1: Start"), "step-1-start");
    }

    #[test]
    fn slugify_empty() {
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn slugify_only_punctuation() {
        assert_eq!(slugify("-_!"), "");
    }

    // ---- create_attribute tests ----

    #[test]
    fn create_attribute_builds_key_value() {
        let attr = create_attribute("href", "https://example.com");
        assert!(matches!(attr.key, AttributeKey::Ident(_)));
        assert!(matches!(attr.value, AttributeValue::LitStr(_)));
    }

    #[test]
    fn with_attributes_wraps_into_attribute_nodes() {
        let attrs = with_attributes(vec![create_attribute("class", "btn")]);
        assert_eq!(attrs.items.len(), 1);
        assert!(matches!(attrs.items[0], AttributeNode::Attribute(_)));
    }

    // ---- parse_code_meta tests ----

    #[test]
    fn code_meta_language_only() {
        let code = markdown::mdast::Code {
            position: None,
            lang: Some("rust".to_string()),
            meta: None,
            value: "fn main() {}".to_string(),
        };
        let meta = parse_code_meta(&code);
        assert_eq!(meta.lang, Some("rust".to_string()));
        assert!(meta.lines.is_none());
        assert!(meta.title.is_none());
        assert!(meta.emphasis.is_empty());
    }

    #[test]
    fn code_meta_with_line_ranges() {
        let code = markdown::mdast::Code {
            position: None,
            lang: Some("python".to_string()),
            meta: Some("{1,3} {5-7}".to_string()),
            value: "print()".to_string(),
        };
        let meta = parse_code_meta(&code);
        assert_eq!(meta.lang, Some("python".to_string()));
        assert_eq!(meta.lines, Some("1,3,5-7".to_string()));
    }

    #[test]
    fn code_meta_with_title() {
        let code = markdown::mdast::Code {
            position: None,
            lang: Some("rust".to_string()),
            meta: Some("title=\"main.rs\"".to_string()),
            value: "fn main() {}".to_string(),
        };
        let meta = parse_code_meta(&code);
        assert_eq!(meta.title, Some("main.rs".to_string()));
    }

    #[test]
    fn code_meta_with_emphasis() {
        let code = markdown::mdast::Code {
            position: None,
            lang: Some("bash".to_string()),
            meta: Some("/sudo/ /password/".to_string()),
            value: "sudo apt install".to_string(),
        };
        let meta = parse_code_meta(&code);
        assert_eq!(
            meta.emphasis,
            vec!["sudo".to_string(), "password".to_string()]
        );
    }

    #[test]
    fn code_meta_combined() {
        let code = markdown::mdast::Code {
            position: None,
            lang: Some("rust".to_string()),
            meta: Some("{1,3} title=\"file.rs\" /TODO/".to_string()),
            value: "fn main() {}".to_string(),
        };
        let meta = parse_code_meta(&code);
        assert_eq!(meta.lang, Some("rust".to_string()));
        assert_eq!(meta.lines, Some("1,3".to_string()));
        assert_eq!(meta.title, Some("file.rs".to_string()));
        assert_eq!(meta.emphasis, vec!["TODO".to_string()]);
    }

    #[test]
    fn code_meta_no_language() {
        let code = markdown::mdast::Code {
            position: None,
            lang: None,
            meta: None,
            value: "text".to_string(),
        };
        let meta = parse_code_meta(&code);
        assert!(meta.lang.is_none());
        assert!(meta.lines.is_none());
        assert!(meta.title.is_none());
    }
}
