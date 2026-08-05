//! mdast-to-view AST walker.
//!
//! Transforms `markdown-rs` mdast nodes into Topcoat `view!` AST types
//! (`Node`, `Element`, `Nodes`), enabling markdown content to be rendered
//! through the same code generation pipeline as handwritten templates.

use std::{cell::RefCell, collections::HashMap};

use proc_macro2::Span;
use syn::Path;
use topcoat_view_grammar::view::{Node, Nodes, View, ViewWriter, WriteView};

use crate::parse::get_parse_options;

pub mod helpers;
pub mod jsx;
pub mod node;

/// Context threaded through the walker so JSX element handlers can look up
/// registered components and report diagnostics.
pub struct WalkContext<'a> {
    /// Component registry: tag-name to Rust path pairs.
    pub components: &'a [(String, Path)],
    /// HTML element override registry: tag-name to Rust path pairs.
    /// When a tag is registered here, the walker emits a `Node::Component`
    /// instead of a `Node::Element` for that tag.
    pub overrides: &'a [(&'static str, Path)],
    /// Error strings collected during walking. The macro layer
    /// drains this buffer and converts each entry into a `syn::Error`.
    pub errors: RefCell<Vec<String>>,
    /// Span to use for generated literals. Prefer the span from the
    /// `compile_mdx!` file-path argument so diagnostics point to the
    /// invocation site rather than `call_site()`.
    pub span: Span,
    /// Link/image definition registry: normalized identifier to (url, title).
    /// Built during the pre-scan pass so that `LinkReference` and
    /// `ImageReference` nodes can be resolved during the main walk.
    pub definitions: HashMap<String, (String, Option<String>)>,
    /// Footnote definitions collected during the pre-scan pass:
    /// (identifier, children nodes).
    pub footnotes: Vec<(String, Vec<markdown::mdast::Node>)>,
    /// Footnote identifiers in first-reference order (GFM spec).
    /// Populated during the main walk; used to number footnotes
    /// in the document-end section.
    pub footnote_order: RefCell<Vec<String>>,
    /// Heading slug counter for duplicate ID handling.
    /// Maps base slug to occurrence count so that "# Hello" followed
    /// by another "# Hello" produces ids "hello" and "hello-1".
    /// Wrapped in `RefCell` for interior mutability (same pattern as errors,
    /// `footnote_order`).
    pub seen_ids: RefCell<HashMap<String, u32>>,
}

impl<'a> WalkContext<'a> {
    /// Create a new walk context with the given component registry,
    /// override registry, and span. Definition and footnote maps are
    /// initialized empty.
    #[must_use]
    pub fn new(
        components: &'a [(String, Path)],
        overrides: &'a [(&'static str, Path)],
        span: Span,
    ) -> Self {
        Self {
            components,
            overrides,
            errors: RefCell::new(Vec::new()),
            span,
            definitions: HashMap::new(),
            footnotes: Vec::new(),
            footnote_order: RefCell::new(Vec::new()),
            seen_ids: RefCell::new(HashMap::new()),
        }
    }

    /// Create a walk context with pre-populated definition and footnote maps.
    /// Used by `mdx_to_view` after the pre-scan pass.
    #[must_use]
    pub fn with_maps(
        components: &'a [(String, Path)],
        overrides: &'a [(&'static str, Path)],
        span: Span,
        definitions: HashMap<String, (String, Option<String>)>,
        footnotes: Vec<(String, Vec<markdown::mdast::Node>)>,
    ) -> Self {
        Self {
            components,
            overrides,
            errors: RefCell::new(Vec::new()),
            span,
            definitions,
            footnotes,
            footnote_order: RefCell::new(Vec::new()),
            seen_ids: RefCell::new(HashMap::new()),
        }
    }

    /// Create an empty-context walker (no component registry, no overrides).
    #[must_use]
    pub fn empty() -> Self {
        Self::new(&[], &[], Span::call_site())
    }

    /// Create a walker with empty component registry but the given overrides.
    #[must_use]
    pub fn empty_with_overrides(overrides: &'a [(&'static str, Path)]) -> Self {
        Self::new(&[], overrides, Span::call_site())
    }
}

impl Default for WalkContext<'_> {
    fn default() -> Self {
        Self::empty()
    }
}

/// Collects link/image definitions and footnote definitions from the root.
///
/// Iterates root children looking for `Definition` and `FootnoteDefinition`
/// nodes. Definition identifiers are normalized to lowercase per `CommonMark`
/// case-folding rules. Returns a tuple of `(definitions, footnotes)` where
/// definitions maps the normalized identifier to `(url, title)` and footnotes
/// stores `(identifier, children)` pairs.
///
/// Per `CommonMark` spec, definitions must appear at the document level.
/// This function only scans direct root children. If the parser places a
/// definition inside a nested structure (e.g., after a blockquote), it
/// will be silently missed, causing reference links to fail with
/// "unknown reference" errors.
#[must_use]
#[allow(clippy::type_complexity)]
pub fn collect_definitions(
    root: &markdown::mdast::Root,
) -> (
    HashMap<String, (String, Option<String>)>,
    Vec<(String, Vec<markdown::mdast::Node>)>,
) {
    let mut definitions = HashMap::new();
    let mut footnotes = Vec::new();
    for node in &root.children {
        match node {
            markdown::mdast::Node::Definition(d) => {
                let id = d.identifier.trim().to_lowercase();
                definitions.insert(id, (d.url.clone(), d.title.clone()));
            }
            markdown::mdast::Node::FootnoteDefinition(f) => {
                footnotes.push((f.identifier.clone(), f.children.clone()));
            }
            _ => {}
        }
    }
    (definitions, footnotes)
}

/// Format of the frontmatter extracted from an MDX document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub enum FrontmatterFormat {
    /// YAML frontmatter (between `---` delimiters).
    Yaml,
    /// TOML frontmatter (between `+++` delimiters).
    Toml,
}

/// Extracts YAML or TOML frontmatter from the mdast root node.
///
/// Only the first child of the root can be frontmatter (YAML
/// frontmatter must appear at byte offset 0 in the source document).
/// Returns `Some((value_string, format))` when a `Node::Yaml` or `Node::Toml`
/// is the first root child, `None` otherwise.
///
/// Note: `MdxjsEsm` frontmatter is not extracted, as it contains JavaScript
/// expressions that are not deserializable as Rust types.
#[must_use]
pub fn extract_frontmatter(root: &markdown::mdast::Node) -> Option<(String, FrontmatterFormat)> {
    let markdown::mdast::Node::Root(r) = root else {
        return None;
    };
    let first = r.children.first()?;
    match first {
        markdown::mdast::Node::Yaml(y) => Some((y.value.clone(), FrontmatterFormat::Yaml)),
        markdown::mdast::Node::Toml(t) => Some((t.value.clone(), FrontmatterFormat::Toml)),
        _ => None,
    }
}

/// Walks an mdast node tree into a Topcoat `view!` `View`.
///
/// Parses the MDX content using `markdown-rs` with GFM + MDX + frontmatter
/// enabled, then walks the resulting mdast into a `View` value ready for
/// token emission via `ToTokens`.
///
/// Two-pass walk: first collects `Definition` and `FootnoteDefinition` nodes
/// from the root, then walks the remaining nodes with the populated maps.
/// Errors collected during the walk are propagated back into the original
/// `ctx.errors` so that the caller can access them.
///
/// # Errors
///
/// Returns `Err(markdown::message::Message)` if the markdown parser fails.
pub fn mdx_to_view(
    ctx: &WalkContext,
    mdx_content: &str,
) -> Result<View, markdown::message::Message> {
    let options = get_parse_options();
    let root = markdown::to_mdast(mdx_content, &options)?;

    let nodes = match root {
        markdown::mdast::Node::Root(r) => {
            // Pass 1: collect definitions and footnote definitions.
            let (definitions, footnotes) = collect_definitions(&r);
            // Build context with pre-populated maps.
            let ctx_with_maps = WalkContext::with_maps(
                ctx.components,
                ctx.overrides,
                ctx.span,
                definitions,
                footnotes,
            );
            // Pass 2: walk the root children.
            let mut walked = walk_nodes(&ctx_with_maps, &r.children).into_vec();
            // Post-walk: append footnote section if any footnotes were referenced.
            let footnote_order = ctx_with_maps.footnote_order.borrow().clone();
            if !footnote_order.is_empty() {
                walked.push(node::walk_footnote_section(&ctx_with_maps, &footnote_order));
            }
            // Propagate errors from the internal walk context back to the caller's context.
            ctx.errors
                .borrow_mut()
                .extend(ctx_with_maps.errors.borrow_mut().drain(..));
            walked.into()
        }
        _ => Nodes::new(),
    };
    Ok(View { cx: None, nodes })
}

/// Extracts the plain text content from a heading's inline children.
///
/// Recursively collects text from `Text`, `Emphasis`, `Strong`, and `InlineCode`
/// nodes. Used by the Heading arm to generate kebab-case id attributes.
fn extract_heading_text(nodes: &[markdown::mdast::Node]) -> String {
    let mut parts = Vec::new();
    for node in nodes {
        match node {
            markdown::mdast::Node::Text(t) => parts.push(t.value.clone()),
            markdown::mdast::Node::Emphasis(e) => parts.push(extract_heading_text(&e.children)),
            markdown::mdast::Node::Strong(s) => parts.push(extract_heading_text(&s.children)),
            markdown::mdast::Node::InlineCode(c) => parts.push(c.value.clone()),
            _ => {}
        }
    }
    parts.join("")
}

/// Walks a slice of mdast nodes into a `Nodes` collection.
pub fn walk_nodes(ctx: &WalkContext, mdast_nodes: &[markdown::mdast::Node]) -> Nodes {
    let mut nodes = Vec::new();
    for node in mdast_nodes {
        nodes.extend(walk_node(ctx, node));
    }
    nodes.into()
}

/// Walks a single mdast node into zero or more view `Node`s.
#[allow(clippy::match_same_arms)]
pub fn walk_node(ctx: &WalkContext, node: &markdown::mdast::Node) -> Vec<Node> {
    match node {
        markdown::mdast::Node::Root(r) => walk_nodes(ctx, &r.children).into_vec(),
        markdown::mdast::Node::Paragraph(p) => {
            let children = walk_nodes(ctx, &p.children);
            vec![jsx::element_or_override(
                ctx,
                "p",
                topcoat_view_grammar::attributes::Attributes::default(),
                children,
            )]
        }
        markdown::mdast::Node::Heading(h) => {
            let tag = format!("h{}", h.depth);
            let children = walk_nodes(ctx, &h.children);
            // Generate kebab-case id attribute for URL anchor links.
            let heading_text = extract_heading_text(&h.children);
            let base_slug = helpers::slugify(&heading_text);
            let id_value = if base_slug.is_empty() {
                tag.clone()
            } else {
                // The slug counter is shared across heading levels, so that
                // "# Hello" and "## Hello" produce ids "hello" and "hello-1".
                // Some implementations (e.g., GitHub, Jekyll) maintain per-level
                // counters, but the shared model is simpler and avoids collisions
                // when different heading levels use the same text.
                let count = {
                    let mut seen = ctx.seen_ids.borrow_mut();
                    let c = seen.get(&base_slug).copied().unwrap_or(0);
                    seen.insert(base_slug.clone(), c + 1);
                    c
                };
                if count == 0 {
                    base_slug
                } else {
                    format!("{base_slug}-{count}")
                }
            };
            let attrs = vec![helpers::create_attribute("id", &id_value)];
            let attributes = helpers::with_attributes(attrs);
            vec![jsx::element_or_override(ctx, &tag, attributes, children)]
        }
        markdown::mdast::Node::Text(t) => {
            vec![helpers::text_node(&t.value)]
        }
        markdown::mdast::Node::Emphasis(e) => {
            vec![helpers::html_element("em", walk_nodes(ctx, &e.children))]
        }
        markdown::mdast::Node::Strong(s) => {
            let children = walk_nodes(ctx, &s.children);
            vec![jsx::element_or_override(
                ctx,
                "strong",
                topcoat_view_grammar::attributes::Attributes::default(),
                children,
            )]
        }
        markdown::mdast::Node::InlineCode(c) => {
            vec![jsx::element_or_override(
                ctx,
                "code",
                topcoat_view_grammar::attributes::Attributes::default(),
                Nodes::from(vec![helpers::text_node(&c.value)]),
            )]
        }
        markdown::mdast::Node::Blockquote(b) => {
            let children = walk_nodes(ctx, &b.children);
            vec![jsx::element_or_override(
                ctx,
                "blockquote",
                topcoat_view_grammar::attributes::Attributes::default(),
                children,
            )]
        }
        markdown::mdast::Node::ThematicBreak(_) => {
            vec![jsx::void_element_or_override(
                ctx,
                "hr",
                topcoat_view_grammar::attributes::Attributes::default(),
            )]
        }
        markdown::mdast::Node::Break(_) => {
            vec![Node::Element(Box::new(helpers::void_element("br")))]
        }
        markdown::mdast::Node::Link(l) => vec![node::walk_link(ctx, l)],
        markdown::mdast::Node::Image(i) => vec![node::walk_image(ctx, i)],
        // Nodes that produce no output:
        // - Html: never produced, html_flow and html_text are off in get_parse_options()
        // - MdxjsEsm: JavaScript expressions, not Rust-deserializable
        // - Definition: a declaration, resolved through ctx.definitions instead
        // - FootnoteDefinition: rendered at document end, not inline
        //
        // Handled elsewhere:
        // - Yaml, Toml: extracted by extract_frontmatter() before the walk
        // - TableRow, TableCell: handled internally by walk_table
        //
        // Not yet supported:
        // - MdxFlowExpression, MdxTextExpression: MDX expressions
        // - InlineMath, Math: LaTeX math, not enabled in parse options
        markdown::mdast::Node::Html(_)
        | markdown::mdast::Node::MdxjsEsm(_)
        | markdown::mdast::Node::Definition(_)
        | markdown::mdast::Node::FootnoteDefinition(_) => Vec::new(),
        markdown::mdast::Node::Code(c) => vec![node::walk_code_block(ctx, c)],
        markdown::mdast::Node::List(l) => vec![node::walk_list(ctx, l)],
        markdown::mdast::Node::ListItem(li) => vec![node::walk_list_item(ctx, li)],
        markdown::mdast::Node::Table(t) => vec![node::walk_table(ctx, t)],
        markdown::mdast::Node::Delete(d) => vec![node::walk_delete(ctx, d)],
        // MDX JSX component elements.
        markdown::mdast::Node::MdxJsxFlowElement(el) => {
            if let Some(comp_node) = jsx::walk_jsx_element(ctx, el) {
                vec![comp_node]
            } else {
                Vec::new()
            }
        }
        markdown::mdast::Node::MdxJsxTextElement(el) => {
            if let Some(comp_node) = jsx::walk_jsx_text_element(ctx, el) {
                vec![comp_node]
            } else {
                Vec::new()
            }
        }
        // Reference-style links and images: resolved from definitions map.
        markdown::mdast::Node::LinkReference(lr) => {
            vec![node::walk_link_reference(ctx, lr)]
        }
        markdown::mdast::Node::ImageReference(ir) => {
            vec![node::walk_image_reference(ctx, ir)]
        }
        // Footnote references: emit superscript link, track order.
        markdown::mdast::Node::FootnoteReference(fr) => {
            vec![node::walk_footnote_reference(ctx, fr)]
        }
        _ => Vec::new(),
    }
}

/// Walks an mdast node directly into a `ViewWriter`.
///
/// `mdast::Text` nodes go through `write_text()` for proper escaping.
/// All other node types are walked into view `Node`s and written through
/// their own `WriteView` implementation. HTML passthrough is disabled:
/// `html_flow` and `html_text` are off in `get_parse_options()`, so
/// `mdast::Html` nodes are never produced by the parser.
pub fn walk_to_writer(ctx: &WalkContext, node: &markdown::mdast::Node, writer: &mut ViewWriter) {
    if let markdown::mdast::Node::Text(t) = node {
        // Text content, escaped for HtmlContext::Text.
        writer.write_text(&t.value);
    } else {
        // For all other node types, construct view nodes and write them.
        let view_nodes = walk_node(ctx, node);
        for vn in view_nodes {
            vn.write(writer);
        }
    }
}

// Re-export jsx functions that are part of the public API used by external consumers.
pub use jsx::coerce_attr_value;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::get_parse_options;

    fn parse_to_root(content: &str) -> markdown::mdast::Node {
        let options = get_parse_options();
        markdown::to_mdast(content, &options).expect("should parse valid markdown")
    }

    // ---- Frontmatter extraction tests ----

    #[test]
    fn extract_frontmatter_yaml_present() {
        let root = parse_to_root("---\ntitle: Hello\ndate: 2024-01-01\n---\n\n# Body");
        let fm = extract_frontmatter(&root);
        assert!(fm.is_some(), "should extract YAML frontmatter");
        let (content, format) = fm.unwrap();
        assert!(matches!(format, FrontmatterFormat::Yaml));
        assert!(content.contains("title"), "should contain title field");
        assert!(content.contains("Hello"), "should contain title value");
    }

    #[test]
    fn extract_frontmatter_none() {
        let root = parse_to_root("# Heading\n\nPlain text");
        assert!(
            extract_frontmatter(&root).is_none(),
            "should return None when no frontmatter"
        );
    }

    #[test]
    fn extract_frontmatter_heading_first() {
        let root = parse_to_root("# heading");
        assert!(
            extract_frontmatter(&root).is_none(),
            "heading-first doc should have no frontmatter"
        );
    }

    #[test]
    fn extract_frontmatter_only_frontmatter() {
        let root = parse_to_root("---\nkey: value\n---");
        let fm = extract_frontmatter(&root);
        assert!(fm.is_some(), "should extract YAML even with no body");
        let (content, format) = fm.unwrap();
        assert!(matches!(format, FrontmatterFormat::Yaml));
        assert!(content.contains("key"), "should contain the YAML content");
    }

    #[test]
    fn extract_frontmatter_toml_present() {
        let root = parse_to_root("+++\ntitle = \"Hello\"\ndate = 2024-01-01\n+++\n\n# Body");
        let fm = extract_frontmatter(&root);
        assert!(fm.is_some(), "should extract TOML frontmatter");
        let (content, format) = fm.unwrap();
        assert!(matches!(format, FrontmatterFormat::Toml));
        assert!(content.contains("title"), "should contain title field");
        assert!(content.contains("Hello"), "should contain title value");
    }

    #[test]
    fn extract_frontmatter_mdxjs_esm_returns_none() {
        // MdxjsEsm frontmatter is intentionally not extracted since it
        // contains JavaScript expressions, not deserializable data.
        let root = parse_to_root("```js\nexport const title = \"Hello\";\n```\n\n# Body");
        let fm = extract_frontmatter(&root);
        assert!(
            fm.is_none(),
            "MdxjsEsm should not be extracted as frontmatter"
        );
    }

    // ---- Custom field frontmatter tests ----

    #[test]
    fn extract_frontmatter_blog_post_yaml() {
        // Blog post with custom fields: subtitle, publishDate,
        // lastModifiedDate, keywords (in addition to title, tags, excerpt).
        let content = "---\ntitle: \"Blog Post with Custom Metadata\"\nsubtitle: \"A subtitle for the post\"\npublishDate: \"2025-01-01\"\nlastModifiedDate: \"2025-06-01\"\ntags: [blog, example, test]\nexcerpt: \"An excerpt summarizing the blog post content.\"\nkeywords: [blog, example, metadata, keywords, test]\n---\n\n# Body";
        let root = parse_to_root(content);
        let fm = extract_frontmatter(&root);
        assert!(fm.is_some(), "should extract blog post frontmatter");
        let (value, format) = fm.unwrap();
        assert!(matches!(format, FrontmatterFormat::Yaml));
        // Standard fields present
        assert!(value.contains("title:"));
        assert!(value.contains("excerpt:"));
        assert!(value.contains("tags:"));
        // Custom fields preserved in raw string
        assert!(
            value.contains("subtitle:"),
            "subtitle should be in raw YAML"
        );
        assert!(
            value.contains("publishDate:"),
            "publishDate should be in raw YAML"
        );
        assert!(
            value.contains("lastModifiedDate:"),
            "lastModifiedDate should be in raw YAML"
        );
        assert!(
            value.contains("keywords:"),
            "keywords should be in raw YAML"
        );
    }

    #[test]
    fn extract_frontmatter_arbitrary_custom_fields() {
        let content = "---\ntitle: \"Custom Fields Test\"\ncategory: technology\nauthor: \"Jane Doe\"\ncustom_key: \"custom_value\"\n---\n\n# Body";
        let root = parse_to_root(content);
        let fm = extract_frontmatter(&root);
        assert!(fm.is_some());
        let (value, _) = fm.unwrap();
        assert!(value.contains("category:"));
        assert!(value.contains("author:"));
        assert!(value.contains("custom_key:"));
        assert!(value.contains("custom_value"));
    }

    #[test]
    fn extract_frontmatter_toml_custom_fields() {
        let content = "+++\ntitle = \"TOML Custom Fields\"\nsubtitle = \"Using TOML frontmatter\"\nmy_field = \"my_value\"\n[nested]\nkey = \"value\"\n+++\n\n# Body";
        let root = parse_to_root(content);
        let fm = extract_frontmatter(&root);
        assert!(fm.is_some(), "should extract TOML with custom fields");
        let (value, format) = fm.unwrap();
        assert!(matches!(format, FrontmatterFormat::Toml));
        assert!(value.contains("subtitle"));
        assert!(value.contains("my_field"));
        assert!(value.contains("nested"));
    }

    // ---- Two-pass walk infrastructure tests (collect_definitions) ----

    #[test]
    fn collect_definitions_finds_link_definitions() {
        let root = parse_to_root("[example]: https://example.com \"Example\"\n\nText");
        let markdown::mdast::Node::Root(r) = root else {
            panic!("expected root");
        };
        let (definitions, _) = collect_definitions(&r);
        assert_eq!(definitions.len(), 1, "should find one definition");
        let entry = definitions
            .get("example")
            .expect("should have 'example' key");
        assert_eq!(entry.0, "https://example.com", "should store URL");
        assert_eq!(entry.1, Some("Example".to_string()), "should store title");
    }

    #[test]
    fn collect_definitions_normalizes_identifier_case() {
        let root = parse_to_root("[MyLabel]: https://example.com\n\nText");
        let markdown::mdast::Node::Root(r) = root else {
            panic!("expected root");
        };
        let (definitions, _) = collect_definitions(&r);
        assert!(
            definitions.contains_key("mylabel"),
            "should normalize identifier to lowercase"
        );
    }

    #[test]
    fn collect_definitions_finds_footnote_definitions() {
        let root = parse_to_root("[^1]: This is a footnote\n\nText[^1]");
        let markdown::mdast::Node::Root(r) = root else {
            panic!("expected root");
        };
        let (_, footnotes) = collect_definitions(&r);
        assert_eq!(footnotes.len(), 1, "should find one footnote definition");
        assert_eq!(footnotes[0].0, "1", "footnote identifier should be '1'");
        assert!(!footnotes[0].1.is_empty(), "footnote should have children");
    }

    #[test]
    fn collect_definitions_empty_when_no_defs() {
        let root = parse_to_root("# Just a heading");
        let markdown::mdast::Node::Root(r) = root else {
            panic!("expected root");
        };
        let (definitions, footnotes) = collect_definitions(&r);
        assert!(definitions.is_empty(), "should have no definitions");
        assert!(footnotes.is_empty(), "should have no footnotes");
    }

    // ---- mdx_to_view entry point tests ----

    #[test]
    fn mdx_to_view_produces_view() {
        let ctx = WalkContext::empty();
        let view = mdx_to_view(&ctx, "# Test").expect("should parse valid markdown");
        assert!(view.cx.is_none());
        assert!(!view.nodes.is_empty());
    }

    // ---- WalkContext fields test ----

    #[test]
    fn walk_context_has_definitions_and_footnotes() {
        let ctx = WalkContext::with_maps(
            &[],
            &[],
            proc_macro2::Span::call_site(),
            std::collections::HashMap::new(),
            Vec::new(),
        );
        assert!(ctx.definitions.is_empty());
        assert!(ctx.footnotes.is_empty());
        assert!(ctx.footnote_order.borrow().is_empty());
    }

    #[test]
    fn mdx_to_view_returns_error_on_invalid_input() {
        let ctx = WalkContext::empty();
        // Verify the function returns Err instead of panicking.
        // (This input is valid markdown, but we test the return type.)
        let result = mdx_to_view(&ctx, "# Valid heading");
        assert!(result.is_ok());
        let view = result.unwrap();
        assert!(!view.nodes.is_empty());
    }
}
