//! Node-specific walkers for mdast node types.
//!
//! Each function handles one mdast node type: links, images, code blocks,
//! lists, tables, strikethrough, etc. They construct `Node` values using
//! the helpers in `helpers.rs` and check for overrides via `jsx.rs`.

use topcoat_view_grammar::{
    attributes::Attributes,
    view::{Node, Nodes},
};

use super::{
    WalkContext,
    helpers::{
        create_attribute, create_attribute_bool, create_attribute_data, html_element,
        normal_element_with_attrs, parse_code_meta, self_closing_element, text_node,
        with_attributes,
    },
    jsx::{element_or_override, void_element_or_override},
};

// ---------------------------------------------------------------------------
// Node-specific walkers
// ---------------------------------------------------------------------------

/// Checks if a URL uses a dangerous protocol (XSS mitigation).
/// Blocks `javascript:`, `vbscript:`, and ALL `data:` URIs (including
/// `data:image/svg+xml` which can execute JS via SVG event handlers).
pub(crate) fn is_safe_url(url: &str) -> bool {
    let cleaned: String = url.chars().filter(|c| !c.is_control()).collect();
    let trimmed = cleaned.trim().to_ascii_lowercase();
    !trimmed.starts_with("javascript:")
        && !trimmed.starts_with("vbscript:")
        && !trimmed.starts_with("data:")
}

/// Walks a link node: `<a href="url" title="...">...</a>`.
/// Strips dangerous URL schemes (javascript:, vbscript:, data:)
/// to prevent XSS, rendering link text as a `<span>` without href.
pub(crate) fn walk_link(ctx: &WalkContext, link: &markdown::mdast::Link) -> Node {
    if !is_safe_url(&link.url) {
        // Strip the href to prevent XSS; render link text only.
        // is_safe_url() check runs BEFORE try_apply_override()
        // so dangerous URLs never route through the override component.
        let children = super::walk_nodes(ctx, &link.children);
        return html_element("span", children);
    }
    let mut attrs = Vec::with_capacity(2);
    attrs.push(create_attribute("href", &link.url));
    if let Some(title) = &link.title {
        attrs.push(create_attribute("title", title));
    }
    let attributes = with_attributes(attrs);
    let children = super::walk_nodes(ctx, &link.children);
    // Check for override AFTER is_safe_url() passes (XSS protection preserved).
    element_or_override(ctx, "a", attributes, children)
}

/// Walks an image node: `<img src="url" alt="alt" title="...">`.
/// Strips dangerous URL schemes (javascript:, vbscript:, data:)
/// to prevent XSS, rendering alt text only without src.
pub(crate) fn walk_image(ctx: &WalkContext, image: &markdown::mdast::Image) -> Node {
    if !is_safe_url(&image.url) {
        // Strip the src to prevent XSS; render alt text only.
        let children = Nodes::from(vec![text_node(&image.alt)]);
        return html_element("span", children);
    }
    let mut attrs = Vec::with_capacity(3);
    attrs.push(create_attribute("src", &image.url));
    attrs.push(create_attribute("alt", &image.alt));
    if let Some(title) = &image.title {
        attrs.push(create_attribute("title", title));
    }
    let attributes = with_attributes(attrs);
    // Check for override before constructing the <img> void element.
    void_element_or_override(ctx, "img", attributes)
}

/// Walks a fenced code block: `<pre><code class="language-{lang}">...</code></pre>`.
///
/// Parses the code block's meta string (language, line ranges, title, emphasis)
/// and attaches them as `data-*` attributes on the `<pre>` element for
/// downstream syntax highlighting components.
pub(crate) fn walk_code_block(ctx: &WalkContext, code: &markdown::mdast::Code) -> Node {
    let meta = parse_code_meta(code);

    // data-* attributes on <pre> from meta string.
    let mut pre_attrs = Vec::new();
    if let Some(ref lang) = meta.lang {
        pre_attrs.push(create_attribute_data("data-lang", lang));
    }
    if let Some(ref lines) = meta.lines {
        pre_attrs.push(create_attribute_data("data-lines", lines));
    }
    if let Some(ref title) = meta.title {
        pre_attrs.push(create_attribute_data("data-title", title));
    }
    if !meta.emphasis.is_empty() {
        pre_attrs.push(create_attribute_data(
            "data-emphasis",
            &meta.emphasis.join(","),
        ));
    }

    // class="language-{lang}" on <code> for backward compatibility.
    let mut code_attrs = Vec::new();
    if let Some(ref lang) = meta.lang {
        code_attrs.push(create_attribute("class", &format!("language-{lang}")));
    }
    let code_attrs = with_attributes(code_attrs);
    let code_children = Nodes::from(vec![text_node(&code.value)]);
    let code_el = normal_element_with_attrs("code", code_attrs, code_children);
    let pre_children = Nodes::from(vec![Node::Element(Box::new(code_el))]);
    let pre_attributes = with_attributes(pre_attrs);

    // Check for override at the <pre> level (outermost element).
    element_or_override(ctx, "pre", pre_attributes, pre_children)
}

/// Walks a list: `<ul>` or `<ol>` with `<li>` children.
pub(crate) fn walk_list(ctx: &WalkContext, list: &markdown::mdast::List) -> Node {
    let tag = if list.ordered { "ol" } else { "ul" };
    let mut children = Vec::new();
    for node in &list.children {
        match node {
            markdown::mdast::Node::ListItem(item) => {
                children.push(walk_list_item(ctx, item));
            }
            other => children.extend(super::walk_node(ctx, other)),
        }
    }
    element_or_override(ctx, tag, Attributes::default(), Nodes::from(children))
}

/// Walks a list item: `<li>` with optional leading checkbox for task lists.
pub(crate) fn walk_list_item(ctx: &WalkContext, item: &markdown::mdast::ListItem) -> Node {
    let mut children = Vec::new();
    if let Some(checked) = &item.checked {
        if *checked {
            // <input type="checkbox" checked disabled />
            let input_attrs = vec![
                create_attribute("type", "checkbox"),
                create_attribute_bool("checked"),
                create_attribute("disabled", ""),
            ];
            let input_el = self_closing_element("input", with_attributes(input_attrs));
            children.push(Node::Element(Box::new(input_el)));
        } else {
            // <input type="checkbox" disabled />, so no checked attribute
            let input_attrs = vec![
                create_attribute("type", "checkbox"),
                create_attribute("disabled", ""),
            ];
            let input_el = self_closing_element("input", with_attributes(input_attrs));
            children.push(Node::Element(Box::new(input_el)));
        }
    }
    children.extend(super::walk_nodes(ctx, &item.children).into_vec());
    element_or_override(ctx, "li", Attributes::default(), Nodes::from(children))
}

/// Walks a table: `<table><thead>...</thead><tbody>...</tbody></table>`.
pub(crate) fn walk_table(ctx: &WalkContext, table: &markdown::mdast::Table) -> Node {
    let mut child_nodes = Vec::new();

    // Iterate over table.children, each of which is a Node::TableRow.
    let row_nodes: Vec<&markdown::mdast::TableRow> = table
        .children
        .iter()
        .filter_map(|n| {
            if let markdown::mdast::Node::TableRow(row) = n {
                Some(row)
            } else {
                None
            }
        })
        .collect();

    // First row is <thead>, rest is <tbody>.
    if let Some(head_row) = row_nodes.first() {
        let th_cells: Vec<Node> = head_row
            .children
            .iter()
            .enumerate()
            .filter_map(|(col_idx, n)| {
                if let markdown::mdast::Node::TableCell(cell) = n {
                    Some(walk_table_cell_inner(
                        ctx,
                        cell,
                        true,
                        col_idx,
                        &table.align,
                    ))
                } else {
                    None
                }
            })
            .collect();
        let tr = html_element("tr", Nodes::from(th_cells));
        let thead = html_element("thead", Nodes::from(vec![tr]));
        child_nodes.push(thead);
    }

    if row_nodes.len() > 1 {
        let body_rows: Vec<Node> = row_nodes[1..]
            .iter()
            .map(|row| {
                let td_cells: Vec<Node> = row
                    .children
                    .iter()
                    .enumerate()
                    .filter_map(|(col_idx, n)| {
                        if let markdown::mdast::Node::TableCell(cell) = n {
                            Some(walk_table_cell_inner(
                                ctx,
                                cell,
                                false,
                                col_idx,
                                &table.align,
                            ))
                        } else {
                            None
                        }
                    })
                    .collect();
                html_element("tr", Nodes::from(td_cells))
            })
            .collect();
        let tbody = html_element("tbody", Nodes::from(body_rows));
        child_nodes.push(tbody);
    }

    element_or_override(
        ctx,
        "table",
        Attributes::default(),
        Nodes::from(child_nodes),
    )
}

/// Walks a table cell: `<th>` or `<td>` with optional alignment style.
pub(crate) fn walk_table_cell_inner(
    ctx: &WalkContext,
    cell: &markdown::mdast::TableCell,
    is_header: bool,
    col_idx: usize,
    align: &[markdown::mdast::AlignKind],
) -> Node {
    let tag = if is_header { "th" } else { "td" };
    let mut attrs = Vec::new();
    // Look up alignment from the table's align vector by column index.
    if let Some(&align_kind) = align.get(col_idx)
        && !matches!(align_kind, markdown::mdast::AlignKind::None)
    {
        let value = match align_kind {
            markdown::mdast::AlignKind::Left => "left",
            markdown::mdast::AlignKind::Right => "right",
            markdown::mdast::AlignKind::Center => "center",
            markdown::mdast::AlignKind::None => unreachable!(),
        };
        attrs.push(create_attribute("style", &format!("text-align: {value}")));
    }
    // Cell children are Node variants (Text, Emphasis, etc.), not TableCell.
    let children = super::walk_nodes(ctx, &cell.children);
    let attributes = with_attributes(attrs);
    element_or_override(ctx, tag, attributes, children)
}

/// Walks a delete (strikethrough) node: `<del>...</del>`.
pub(crate) fn walk_delete(ctx: &WalkContext, delete: &markdown::mdast::Delete) -> Node {
    let children = super::walk_nodes(ctx, &delete.children);
    html_element("del", children)
}

/// Walks a link reference node: `[text][ref]` resolved from definitions map.
///
/// Looks up the normalized identifier in `ctx.definitions`. If found and the
/// URL passes `is_safe_url()`, emits an `<a>` element with href.
/// If the URL fails `is_safe_url()`, emits `<span>` with text only.
/// If the definition is not found, pushes an error to `ctx.errors`.
pub(crate) fn walk_link_reference(
    ctx: &WalkContext,
    link_ref: &markdown::mdast::LinkReference,
) -> Node {
    let id = link_ref.identifier.trim().to_lowercase();
    if let Some((url, title)) = ctx.definitions.get(&id) {
        // XSS protection: check URL before emitting <a>.
        if !is_safe_url(url) {
            let children = super::walk_nodes(ctx, &link_ref.children);
            return html_element("span", children);
        }
        let mut attrs = Vec::with_capacity(2);
        attrs.push(create_attribute("href", url));
        if let Some(t) = title {
            attrs.push(create_attribute("title", t));
        }
        let attributes = with_attributes(attrs);
        let children = super::walk_nodes(ctx, &link_ref.children);
        // Check for override AFTER is_safe_url() passes (XSS protection preserved).
        return element_or_override(ctx, "a", attributes, children);
    }
    // Unknown reference: emit a compile-time error.
    ctx.errors.borrow_mut().push(format!(
        "unknown reference link target: '{}'",
        link_ref.identifier
    ));
    // Render all link children as plain inline content, wrapped in a <span>
    // to preserve them. Falls back to the identifier text if there
    // are no children.
    let walked = super::walk_nodes(ctx, &link_ref.children);
    if walked.is_empty() {
        text_node(&link_ref.identifier)
    } else {
        html_element("span", walked)
    }
}

/// Walks an image reference node: `![alt][ref]` resolved from definitions map.
///
/// Looks up the normalized identifier in `ctx.definitions`. If found and the
/// URL passes `is_safe_url()`, emits an `<img>` void element.
/// If the URL fails `is_safe_url()`, emits `<span>` with alt text only.
/// If the definition is not found, pushes an error to `ctx.errors`.
pub(crate) fn walk_image_reference(
    ctx: &WalkContext,
    img_ref: &markdown::mdast::ImageReference,
) -> Node {
    let id = img_ref.identifier.trim().to_lowercase();
    if let Some((url, title)) = ctx.definitions.get(&id) {
        // XSS protection: check URL before emitting <img>.
        if !is_safe_url(url) {
            return html_element("span", Nodes::from(vec![text_node(img_ref.alt.as_str())]));
        }
        let mut attrs = Vec::with_capacity(3);
        attrs.push(create_attribute("src", url));
        attrs.push(create_attribute("alt", img_ref.alt.as_str()));
        if let Some(t) = title {
            attrs.push(create_attribute("title", t));
        }
        let attributes = with_attributes(attrs);
        // Check for override before constructing the <img> void element.
        return void_element_or_override(ctx, "img", attributes);
    }
    // Unknown reference: emit a compile-time error.
    ctx.errors.borrow_mut().push(format!(
        "unknown reference image target: '{}'",
        img_ref.identifier
    ));
    // Render alt text as fallback.
    html_element("span", Nodes::from(vec![text_node(img_ref.alt.as_str())]))
}

/// Walks a footnote reference node: `[^id]` rendered as superscript link.
///
/// Tracks the identifier in `ctx.footnote_order` if not already present
/// (GFM first-reference order). Emits
/// `<sup><a id="fnref-{id}" href="#fn-{id}">{id}</a></sup>` as inline content.
/// The `id` is the target that the footnote section's back-reference links to.
pub(crate) fn walk_footnote_reference(
    ctx: &WalkContext,
    foot_ref: &markdown::mdast::FootnoteReference,
) -> Node {
    // Track first-reference order for GFM numbering.
    {
        let mut order = ctx.footnote_order.borrow_mut();
        if !order.iter().any(|id| id == &foot_ref.identifier) {
            order.push(foot_ref.identifier.clone());
        }
    }
    let anchor_id = format!("fnref-{}", foot_ref.identifier);
    let href = format!("#fn-{}", foot_ref.identifier);
    let attrs = with_attributes(vec![
        create_attribute("id", &anchor_id),
        create_attribute("href", &href),
    ]);
    let link_text = text_node(&foot_ref.identifier);
    let a = Node::Element(Box::new(normal_element_with_attrs(
        "a",
        attrs,
        Nodes::from(vec![link_text]),
    )));
    html_element("sup", Nodes::from(vec![a]))
}

/// Renders a footnote section: `<ol>` with footnote items at document end.
///
/// Called after the main walk by `mdx_to_view` when footnotes were referenced.
/// Each `<li>` contains the footnote definition content with a back-reference link.
/// Numbering follows first-reference order per GFM spec.
pub fn walk_footnote_section(ctx: &WalkContext, footnote_order: &[String]) -> Node {
    let mut li_nodes = Vec::new();
    for id in footnote_order {
        // Find the footnote definition content.
        let content = ctx
            .footnotes
            .iter()
            .find(|(fid, _)| fid == id)
            .map(|(_, children)| super::walk_nodes(ctx, children));
        let content = content.unwrap_or_else(Nodes::new);
        // Build back-reference link: <a href="#fnref-{id}">.
        let back_ref_href = format!("#fnref-{id}");
        let back_ref_attrs = with_attributes(vec![create_attribute("href", &back_ref_href)]);
        let back_ref_text = text_node(id);
        let back_ref = Node::Element(Box::new(normal_element_with_attrs(
            "a",
            back_ref_attrs,
            Nodes::from(vec![back_ref_text]),
        )));
        // Wrap content in <p> if not already a paragraph-like node.
        let p_content = if content.len() == 1
            && matches!(&content[0], Node::Element(e) if e.name().string_name().as_deref() == Some("p"))
        {
            content
        } else {
            Nodes::from(vec![html_element("p", content)])
        };
        // Build <li id="fn-{id}"> content + back_ref </li>.
        let mut li_children = p_content.into_vec();
        li_children.push(back_ref);
        let li_attrs = with_attributes(vec![create_attribute("id", &format!("fn-{id}"))]);
        let li = Node::Element(Box::new(normal_element_with_attrs(
            "li",
            li_attrs,
            Nodes::from(li_children),
        )));
        li_nodes.push(li);
    }
    html_element("ol", Nodes::from(li_nodes))
}

#[cfg(test)]
mod tests {
    use topcoat_view_grammar::view::Element as ViewElement;

    use super::*;
    use crate::parse::get_parse_options;

    fn parse_and_walk(content: &str) -> Nodes {
        parse_and_walk_ctx(&WalkContext::empty(), content)
    }

    fn parse_and_walk_ctx(ctx: &WalkContext, content: &str) -> Nodes {
        let options = get_parse_options();
        let root = markdown::to_mdast(content, &options).unwrap();
        match root {
            markdown::mdast::Node::Root(r) => super::super::walk_nodes(ctx, &r.children),
            _ => unreachable!(),
        }
    }

    /// Collects every top-level element with the given tag name.
    fn find_elements<'a>(nodes: &'a [Node], tag: &str) -> Vec<&'a ViewElement> {
        nodes
            .iter()
            .filter_map(|n| match n {
                Node::Element(e) if e.name().string_name().as_deref() == Some(tag) => {
                    Some(e.as_ref())
                }
                _ => None,
            })
            .collect()
    }

    /// Returns the first top-level element with the given tag name.
    fn find_element<'a>(nodes: &'a [Node], tag: &str) -> Option<&'a ViewElement> {
        find_elements(nodes, tag).into_iter().next()
    }

    // ---- Existing tests ----

    #[test]
    fn walks_heading() {
        let nodes = parse_and_walk("# Hello");
        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert!(
            matches!(node, Node::Element(e) if e.name().string_name().as_deref() == Some("h1")),
            "expected h1 element",
        );
    }

    #[test]
    fn walks_paragraph() {
        let nodes = parse_and_walk("Plain text");
        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert!(
            matches!(node, Node::Element(e) if e.name().string_name().as_deref() == Some("p")),
            "expected p element",
        );
    }

    #[test]
    fn walks_text_inside_paragraph() {
        let nodes = parse_and_walk("Hello world");
        let paragraph = &nodes[0];
        if let Node::Element(e) = paragraph {
            assert_eq!(e.children().len(), 1);
            assert!(matches!(&e.children()[0], Node::Text(_)));
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn walks_thematic_break() {
        // Use "***" instead of "---" to avoid ambiguity with frontmatter
        // (which is enabled in parse options). Stars cannot be parsed as
        // frontmatter, making this test unambiguous.
        let nodes = parse_and_walk("\n***\n");
        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert!(
            matches!(node, Node::Element(e) if matches!(e.as_ref(), ViewElement::Void { .. })),
            "expected void element for thematic break",
        );
        // Also verify it's an <hr>.
        if let Node::Element(e) = node {
            assert_eq!(
                e.name().string_name().as_deref(),
                Some("hr"),
                "thematic break should render as <hr>"
            );
        }
    }

    #[test]
    fn walks_emphasis() {
        let nodes = parse_and_walk("*italic*");
        // Italic appears inside a paragraph
        assert_eq!(nodes.len(), 1);
        if let Node::Element(e) = &nodes[0] {
            assert!(!e.children().is_empty());
            let has_em = e.children().iter().any(|child| {
                if let Node::Element(inner) = child {
                    inner.name().string_name().as_deref() == Some("em")
                } else {
                    false
                }
            });
            assert!(has_em, "paragraph should contain <em> element");
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn walks_strong() {
        let nodes = parse_and_walk("**bold**");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(e) = &nodes[0] {
            let has_strong = e.children().iter().any(|child| {
                if let Node::Element(inner) = child {
                    inner.name().string_name().as_deref() == Some("strong")
                } else {
                    false
                }
            });
            assert!(has_strong, "paragraph should contain <strong> element");
        }
    }

    #[test]
    fn walks_inline_code() {
        let nodes = parse_and_walk("`code`");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(e) = &nodes[0] {
            let has_code = e.children().iter().any(|child| {
                if let Node::Element(inner) = child {
                    inner.name().string_name().as_deref() == Some("code")
                } else {
                    false
                }
            });
            assert!(has_code, "paragraph should contain <code> element");
        }
    }

    #[test]
    fn walks_blockquote() {
        let nodes = parse_and_walk("> quoted");
        assert_eq!(nodes.len(), 1);
        let node = &nodes[0];
        assert!(
            matches!(node, Node::Element(e) if e.name().string_name().as_deref() == Some("blockquote")),
            "expected blockquote element",
        );
    }

    #[test]
    fn walks_break() {
        let nodes = parse_and_walk("line1  \nline2");
        assert_eq!(nodes.len(), 1);
        // Verify the <br> is present inside the paragraph.
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            let has_br = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    matches!(e.as_ref(), ViewElement::Void { .. })
                        && e.name().string_name().as_deref() == Some("br")
                } else {
                    false
                }
            });
            assert!(has_br, "paragraph should contain <br> for hard break");
        } else {
            panic!("expected paragraph element");
        }
    }

    // ---- Tests: URL sanitization (is_safe_url) ----

    #[test]
    fn is_safe_url_allows_http() {
        assert!(is_safe_url("https://example.com"));
        assert!(is_safe_url("http://example.com"));
    }

    #[test]
    fn is_safe_url_allows_relative() {
        assert!(is_safe_url("/path/to/page"));
        assert!(is_safe_url("image.png"));
        assert!(is_safe_url("./relative.md"));
    }

    #[test]
    fn is_safe_url_blocks_javascript() {
        assert!(!is_safe_url("javascript:alert(1)"));
        assert!(!is_safe_url("  javascript:alert(1)"));
        assert!(!is_safe_url("JavaScript:alert(1)"));
    }

    #[test]
    fn is_safe_url_blocks_null_byte_bypass() {
        // Null bytes before the scheme would bypass trim_start().
        assert!(!is_safe_url("\x00javascript:alert(1)"));
        assert!(!is_safe_url("java\x00script:alert(1)"));
        assert!(!is_safe_url("\x00\x00vbscript:msgBox(1)"));
        assert!(!is_safe_url("\x00data:text/html,<script>"));
    }

    #[test]
    fn is_safe_url_blocks_c0_control_bypass() {
        // C0 control chars (tab, CR, etc.) inside the scheme bypass filters
        // that only strip null bytes. Browsers strip C0 controls per the
        // WHATWG URL Standard.
        assert!(!is_safe_url("java\tscript:alert(1)"));
        assert!(!is_safe_url("java\rscript:alert(1)"));
        assert!(!is_safe_url("java\nscript:alert(1)"));
        assert!(!is_safe_url("\tjavascript:alert(1)"));
        assert!(!is_safe_url("\rvbscript:msgBox(1)"));
        assert!(!is_safe_url("\ndata:text/html,evil"));
    }

    #[test]
    fn is_safe_url_blocks_vbscript() {
        assert!(!is_safe_url("vbscript:msgBox(1)"));
    }

    #[test]
    fn is_safe_url_blocks_all_data_uris() {
        // Block data:text/html
        assert!(!is_safe_url("data:text/html,<script>alert(1)</script>"));
        // Block data:image/svg+xml (XSS via SVG event handlers)
        assert!(!is_safe_url("data:image/svg+xml,<svg onload=alert(1)>"));
        // Block base64-encoded SVG
        assert!(!is_safe_url("data:image/svg+xml;base64,PHN2ZyBvbmxvYWQ+"));
        // Block data:text/plain (defense in depth)
        assert!(!is_safe_url("data:text/plain,hello"));
    }

    #[test]
    fn blocks_javascript_url_in_link() {
        let nodes = parse_and_walk("[click](javascript:alert(1))");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            // The link should NOT render as <a>; it should be stripped to <span>.
            let has_a = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("a")
                } else {
                    false
                }
            });
            assert!(!has_a, "javascript: link should NOT produce <a> element");
        }
    }

    #[test]
    fn blocks_data_uri_in_image() {
        // Use a base64 data URI to avoid <svg> being parsed as JSX after HTML disable.
        let nodes = parse_and_walk("![x](data:image/svg+xml;base64,PHN2Zw==)");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            let has_img = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("img")
                } else {
                    false
                }
            });
            assert!(!has_img, "data: URI image should NOT produce <img> element");
        }
    }

    // ---- New tests: links and images ----

    #[test]
    fn walks_link() {
        let nodes = parse_and_walk("[text](https://example.com)");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            let has_a = p.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("a")
                } else {
                    false
                }
            });
            assert!(has_a, "paragraph should contain <a> element");
        }
    }

    #[test]
    fn walks_link_with_href_attribute() {
        let nodes = parse_and_walk("[link](https://example.com)");
        if let Node::Element(p) = &nodes[0] {
            let a = p.children().iter().find_map(|c| {
                if let Node::Element(e) = c {
                    if e.name().string_name().as_deref() == Some("a") {
                        Some(e.as_ref())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            assert!(a.is_some(), "should find <a> element");
            let a = a.unwrap();
            let attrs = a.attributes();
            assert!(!attrs.is_empty(), "link should have attributes");
        }
    }

    #[test]
    fn walks_image() {
        let nodes = parse_and_walk("![alt](photo.png)");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            let has_img = p.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    matches!(inner.as_ref(), ViewElement::Void { .. })
                        && inner.name().string_name().as_deref() == Some("img")
                } else {
                    false
                }
            });
            assert!(has_img, "paragraph should contain <img> void element");
        }
    }

    #[test]
    fn walks_image_with_src_and_alt() {
        let nodes = parse_and_walk("![Photo](photo.png)");
        if let Node::Element(p) = &nodes[0] {
            let img = p.children().iter().find_map(|c| {
                if let Node::Element(e) = c {
                    if e.name().string_name().as_deref() == Some("img") {
                        Some(e.as_ref())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            assert!(img.is_some(), "should find <img> element");
            let img = img.unwrap();
            let attrs = img.attributes();
            assert!(!attrs.is_empty(), "image should have attributes (src, alt)");
        }
    }

    // ---- New tests: code blocks ----

    #[test]
    fn walks_code_block_with_language() {
        let nodes = parse_and_walk("```rust\nfn main() {}\n```");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(pre) = &nodes[0] {
            assert_eq!(
                pre.name().string_name().as_deref(),
                Some("pre"),
                "should be <pre> element"
            );
            let has_code = pre.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("code")
                } else {
                    false
                }
            });
            assert!(has_code, "<pre> should contain <code>");
            // Check code has class attribute
            let code = pre
                .children()
                .iter()
                .find_map(|c| {
                    if let Node::Element(e) = c {
                        if e.name().string_name().as_deref() == Some("code") {
                            Some(e.as_ref())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .unwrap();
            assert!(
                !code.attributes().is_empty(),
                "code should have class attribute for language",
            );
        }
    }

    #[test]
    fn walks_code_block_without_language() {
        let nodes = parse_and_walk("```\nno lang\n```");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(pre) = &nodes[0] {
            assert_eq!(
                pre.name().string_name().as_deref(),
                Some("pre"),
                "should be <pre> element"
            );
        }
    }

    // ---- New tests: lists ----

    #[test]
    fn walks_ordered_list() {
        let nodes = parse_and_walk("1. first");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(ol) = &nodes[0] {
            assert_eq!(
                ol.name().string_name().as_deref(),
                Some("ol"),
                "should be <ol> element"
            );
        }
    }

    #[test]
    fn walks_unordered_list() {
        let nodes = parse_and_walk("- item");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(ul) = &nodes[0] {
            assert_eq!(
                ul.name().string_name().as_deref(),
                Some("ul"),
                "should be <ul> element"
            );
            let has_li = ul.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("li")
                } else {
                    false
                }
            });
            assert!(has_li, "<ul> should contain <li>");
        }
    }

    #[test]
    fn walks_task_list_checked() {
        let nodes = parse_and_walk("- [x] done");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(ul) = &nodes[0] {
            // The <li> should have a self-closing <input> as first child.
            let li = ul.children().iter().find_map(|c| {
                if let Node::Element(e) = c {
                    if e.name().string_name().as_deref() == Some("li") {
                        Some(e.as_ref())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            assert!(li.is_some(), "<ul> should contain <li>");
            let li = li.unwrap();
            assert!(
                !li.children().is_empty(),
                "<li> should contain checkbox input",
            );
        }
    }

    #[test]
    fn walks_task_list_unchecked() {
        let nodes = parse_and_walk("- [ ] pending");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(ul) = &nodes[0] {
            let li = ul.children().iter().find_map(|c| {
                if let Node::Element(e) = c {
                    if e.name().string_name().as_deref() == Some("li") {
                        Some(e.as_ref())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            assert!(li.is_some(), "<ul> should contain <li>");
        }
    }

    #[test]
    fn walks_unordered_list_no_checkbox() {
        // Regular list item (not a task list) should NOT have a checkbox.
        let nodes = parse_and_walk("- regular item");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(ul) = &nodes[0] {
            let li = ul.children().iter().find_map(|c| {
                if let Node::Element(e) = c {
                    if e.name().string_name().as_deref() == Some("li") {
                        Some(e.as_ref())
                    } else {
                        None
                    }
                } else {
                    None
                }
            });
            assert!(li.is_some(), "<ul> should contain <li>");
            let li = li.unwrap();
            // First child should be text/paragraph, not an <input>.
            let first_is_input = li.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("input")
                } else {
                    false
                }
            });
            assert!(
                !first_is_input,
                "regular list item should NOT contain <input> checkbox"
            );
        }
    }

    // ---- New tests: tables ----

    #[test]
    fn walks_table() {
        let nodes = parse_and_walk("| A | B |\n|---|---|\n| 1 | 2 |");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(table) = &nodes[0] {
            assert_eq!(
                table.name().string_name().as_deref(),
                Some("table"),
                "should be <table>"
            );
            // Should have thead and tbody children
            let has_thead = table.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("thead")
                } else {
                    false
                }
            });
            let has_tbody = table.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("tbody")
                } else {
                    false
                }
            });
            assert!(has_thead, "table should have <thead>");
            assert!(has_tbody, "table should have <tbody>");
        }
    }

    #[test]
    fn walks_table_with_alignment() {
        let nodes = parse_and_walk("| A |\n|:---|\n| 1 |");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(table) = &nodes[0] {
            // Find <th> element
            let th = find_element_recursive(table, "th");
            assert!(th.is_some(), "table should have <th>");
            let th = th.unwrap();
            assert!(
                !th.attributes().is_empty(),
                "aligned <th> should have style attribute"
            );
        }
    }

    // ---- New tests: strikethrough ----

    #[test]
    fn walks_strikethrough() {
        let nodes = parse_and_walk("~~deleted~~");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            let has_del = p.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("del")
                } else {
                    false
                }
            });
            assert!(has_del, "paragraph should contain <del> element");
        }
    }

    // ---- New tests: heading depth ----

    #[test]
    fn walks_heading_depth_1_to_6() {
        for (depth, prefix) in [
            (1, "#"),
            (2, "##"),
            (3, "###"),
            (4, "####"),
            (5, "#####"),
            (6, "######"),
        ] {
            let content = format!("{prefix} Level {depth}");
            let nodes = parse_and_walk(&content);
            assert_eq!(nodes.len(), 1, "depth {depth}: expected one node");
            if let Node::Element(e) = &nodes[0] {
                let expected = format!("h{depth}");
                assert_eq!(
                    e.name().string_name().as_deref(),
                    Some(expected.as_str()),
                    "depth {depth}: expected {expected}",
                );
            } else {
                panic!("depth {depth}: expected element");
            }
        }
    }

    // ---- New tests: walk_node dispatch ----

    #[test]
    fn walk_node_dispatches_code_block() {
        // mdast::Code (fenced block) should produce <pre>, not <code> at top.
        let nodes = parse_and_walk("```python\nprint()\n```");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("pre")),
            "code block should produce <pre>",
        );
    }

    #[test]
    fn walk_node_dispatches_inline_code() {
        // mdast::InlineCode should produce <code> inside paragraph.
        let nodes = parse_and_walk("`inline`");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            let has_code = p.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("code")
                } else {
                    false
                }
            });
            assert!(has_code, "inline code should produce <code>");
        }
    }

    #[test]
    fn walk_node_dispatches_image_not_link() {
        // Image syntax ![alt](src) should produce <img>, not <a>.
        let nodes = parse_and_walk("![photo](image.png)");
        assert_eq!(nodes.len(), 1);
        if let Node::Element(p) = &nodes[0] {
            let has_img = p.children().iter().any(|c| {
                if let Node::Element(inner) = c {
                    inner.name().string_name().as_deref() == Some("img")
                } else {
                    false
                }
            });
            assert!(has_img, "image should produce <img>");
        }
    }

    #[test]
    fn walk_node_dispatches_table() {
        let nodes = parse_and_walk("| Col |\n|-----|\n| Val |");
        assert_eq!(nodes.len(), 1);
        assert!(
            matches!(&nodes[0], Node::Element(e) if e.name().string_name().as_deref() == Some("table")),
            "table should produce <table>",
        );
    }

    // ---- Code block meta string tests ----

    #[test]
    fn code_meta_emits_data_attributes() {
        // ```rust {1,3} title="file.rs" should emit data-lang, data-lines, data-title on <pre>
        let ctx = WalkContext::empty();
        let view =
            super::super::mdx_to_view(&ctx, "```rust {1,3} title=\"file.rs\"\nfn main() {}\n```")
                .expect("should parse");
        assert!(!view.nodes.is_empty());
        let pre = find_element(&view.nodes, "pre").expect("should have pre element");
        let data_lang = find_attr_value(pre, "data-lang");
        let data_lines = find_attr_value(pre, "data-lines");
        let data_title = find_attr_value(pre, "data-title");
        assert_eq!(
            data_lang,
            Some("rust".to_string()),
            "should have data-lang=\"rust\""
        );
        assert_eq!(
            data_lines,
            Some("1,3".to_string()),
            "should have data-lines=\"1,3\""
        );
        assert_eq!(
            data_title,
            Some("file.rs".to_string()),
            "should have data-title=\"file.rs\""
        );
    }

    #[test]
    fn code_meta_language_only() {
        // ```python should emit only data-lang
        let ctx = WalkContext::empty();
        let view =
            super::super::mdx_to_view(&ctx, "```python\nprint()\n```").expect("should parse");
        let pre = find_element(&view.nodes, "pre").expect("should have pre element");
        let data_lang = find_attr_value(pre, "data-lang");
        assert_eq!(data_lang, Some("python".to_string()));
    }

    #[test]
    fn code_meta_no_lang_no_attrs() {
        // ``` (no language) should emit no data-* attributes
        let ctx = WalkContext::empty();
        let view = super::super::mdx_to_view(&ctx, "```\nno lang\n```").expect("should parse");
        let pre = find_element(&view.nodes, "pre").expect("should have pre element");
        let data_lang = find_attr_value(pre, "data-lang");
        assert!(
            data_lang.is_none(),
            "should have no data-lang when no language"
        );
    }

    #[test]
    fn code_meta_emphasis() {
        // ```bash /sudo/ should emit data-emphasis on <pre>
        let ctx = WalkContext::empty();
        let view =
            super::super::mdx_to_view(&ctx, "```bash /sudo/\necho hi\n```").expect("should parse");
        let pre = find_element(&view.nodes, "pre").expect("should have pre element");
        let data_emphasis = find_attr_value(pre, "data-emphasis");
        assert_eq!(
            data_emphasis,
            Some("sudo".to_string()),
            "should have data-emphasis=\"sudo\""
        );
    }

    // ---- Heading ID tests ----

    /// Find the value of a named attribute on an element.
    fn find_attr_value(element: &ViewElement, name: &str) -> Option<String> {
        for item in &element.attributes().items {
            if let topcoat_view_grammar::attributes::AttributeNode::Attribute(attr) = item {
                // Build the full attribute key name (handles hyphenated keys like data-lang).
                let key_name = if let topcoat_view_grammar::attributes::AttributeKey::Ident(id) =
                    &attr.key
                {
                    let rest_parts: Vec<String> = id
                        .rest
                        .iter()
                        .map(|seg| match &seg.part {
                            topcoat_view_grammar::view::HtmlIdentPart::Ident(i) => i.to_string(),
                            topcoat_view_grammar::view::HtmlIdentPart::Int(lit) => {
                                lit.base10_parse::<u64>().unwrap_or(0).to_string()
                            }
                        })
                        .collect();
                    if rest_parts.is_empty() {
                        id.first.to_string()
                    } else {
                        format!("{}-{}", id.first, rest_parts.join("-"))
                    }
                } else {
                    continue;
                };
                if key_name == name
                    && let topcoat_view_grammar::attributes::AttributeValue::LitStr(lit) =
                        &attr.value
                {
                    return Some(lit.value());
                }
            }
        }
        None
    }

    #[test]
    fn heading_id_simple() {
        // "# Hello" should produce <h1 id="hello">
        let ctx = WalkContext::empty();
        let view = super::super::mdx_to_view(&ctx, "# Hello").expect("should parse");
        assert!(!view.nodes.is_empty());
        let h1 = find_element(&view.nodes, "h1").expect("should have h1 element");
        let id_value = find_attr_value(h1, "id");
        assert_eq!(
            id_value,
            Some("hello".to_string()),
            "heading should have id=\"hello\""
        );
    }

    #[test]
    fn heading_id_duplicate() {
        // Two "# Hello" headings should produce id="hello" and id="hello-1"
        let ctx = WalkContext::empty();
        let view = super::super::mdx_to_view(&ctx, "# Hello\n\n# Hello").expect("should parse");
        let h1s = find_elements(&view.nodes, "h1");
        assert_eq!(h1s.len(), 2, "should have two h1 elements");
        let id1 = find_attr_value(h1s[0], "id");
        let id2 = find_attr_value(h1s[1], "id");
        assert_eq!(
            id1,
            Some("hello".to_string()),
            "first heading should have id=\"hello\""
        );
        assert_eq!(
            id2,
            Some("hello-1".to_string()),
            "second heading should have id=\"hello-1\""
        );
    }

    // ---- Helper for recursive element finding ----

    fn find_element_recursive<'a>(element: &'a ViewElement, tag: &str) -> Option<&'a ViewElement> {
        if element.name().string_name().as_deref() == Some(tag) {
            return Some(element);
        }
        for child in element.children() {
            if let Node::Element(inner) = child
                && let Some(found) = find_element_recursive(inner, tag)
            {
                return Some(found);
            }
        }
        None
    }

    // ---- Helper for parsing with two-pass walk (reference links, footnotes) ----

    fn parse_and_walk_full_ctx(
        ctx: &WalkContext,
        content: &str,
    ) -> Result<topcoat_view_grammar::view::View, markdown::message::Message> {
        super::super::mdx_to_view(ctx, content)
    }

    // ---- Reference link tests ----

    #[test]
    fn reference_link_resolves_to_anchor() {
        // [text][ref] should resolve to <a href="url">text</a> when Definition exists.
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(
            &ctx,
            "[click here][example]\n\n[example]: https://example.com",
        )
        .expect("should parse");
        assert!(!view.nodes.is_empty());
        // The paragraph should contain an <a> element.
        if let Node::Element(p) = &view.nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            let has_a = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("a")
                } else {
                    false
                }
            });
            assert!(has_a, "reference link should resolve to <a> element");
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn reference_image_resolves_to_img() {
        // ![alt][ref] should resolve to <img src="url" alt="alt"> when Definition exists.
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(&ctx, "![photo][img-ref]\n\n[img-ref]: photo.png")
            .expect("should parse");
        assert!(!view.nodes.is_empty());
        // The paragraph should contain an <img> void element.
        if let Node::Element(p) = &view.nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            let has_img = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("img")
                } else {
                    false
                }
            });
            assert!(has_img, "reference image should resolve to <img> element");
        } else {
            panic!("expected paragraph element");
        }
    }

    #[test]
    fn unknown_ref_emits_error() {
        // [text][undefined] should push an error to WalkContext.errors.
        //
        // Note: markdown-rs resolves reference links at parse time. When a
        // LinkReference has no matching Definition in the mdast, the parser
        // renders it as literal text "[click][missing]" instead of a
        // LinkReference node. The walker only sees LinkReference nodes when
        // the parser found a matching Definition.
        //
        // This test verifies the defensive error path in walk_link_reference:
        // when a LinkReference node reaches the walker but its normalized id
        // is not in ctx.definitions (e.g. due to a case mismatch or collection
        // ordering), an error is emitted and fallback text is rendered.
        //
        // We construct a WalkContext with a definitions map missing the
        // target key to exercise this path.
        let mut defs = std::collections::HashMap::new();
        defs.insert("other".to_string(), ("https://other.com".to_string(), None));
        // "missing" is intentionally not in the map.
        let ctx =
            WalkContext::with_maps(&[], &[], proc_macro2::Span::call_site(), defs, Vec::new());
        // Build a LinkReference mdast node manually and walk it.
        let link_ref = markdown::mdast::LinkReference {
            children: vec![markdown::mdast::Node::Text(markdown::mdast::Text {
                position: None,
                value: "click".to_string(),
            })],
            position: None,
            reference_kind: markdown::mdast::ReferenceKind::Full,
            identifier: "missing".to_string(),
            label: None,
        };
        let _node = super::walk_link_reference(&ctx, &link_ref);
        let errors = ctx.errors.borrow();
        assert!(
            !errors.is_empty(),
            "should emit error for unknown reference target, errors: {:?}",
            *errors
        );
        assert!(
            errors.iter().any(|e| e.contains("missing")),
            "error should mention the missing identifier: {:?}",
            *errors
        );
    }

    #[test]
    fn reference_link_blocks_xss() {
        // Definition URL with javascript: should NOT produce <a>.
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(&ctx, "[xss][bad]\n\n[bad]: javascript:alert(1)")
            .expect("should parse");
        assert!(!view.nodes.is_empty());
        // Should NOT contain an <a> element.
        if let Node::Element(p) = &view.nodes[0] {
            assert_eq!(p.name().string_name().as_deref(), Some("p"));
            let has_a = p.children().iter().any(|c| {
                if let Node::Element(e) = c {
                    e.name().string_name().as_deref() == Some("a")
                } else {
                    false
                }
            });
            assert!(!has_a, "javascript: reference link should NOT produce <a>");
        }
    }

    #[test]
    fn definition_skipped_during_walk() {
        // Definition nodes should NOT appear as rendered content.
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(&ctx, "[example]: https://example.com\n\nBody text")
            .expect("should parse");
        // The view should contain only the body text paragraph, not the definition.
        // The view should have content, but not a rendered definition.
        assert!(
            view.nodes.len() <= 1,
            "definition should not produce extra nodes, got {}",
            view.nodes.len()
        );
    }

    // ---- Footnote tests ----

    #[test]
    fn footnote_reference_renders_as_superscript() {
        // [^1] should render as <sup><a href="#fn-1">1</a></sup>
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(&ctx, "See note[^1].\n\n[^1]: This is a footnote.")
            .expect("should parse");
        // The paragraph should contain a <sup> element.
        assert!(!view.nodes.is_empty());
        let has_sup = view.nodes.iter().any(|n| {
            if let Node::Element(p) = n {
                p.name().string_name().as_deref() == Some("p")
                    && p.children().iter().any(|c| {
                        if let Node::Element(e) = c {
                            e.name().string_name().as_deref() == Some("sup")
                        } else {
                            false
                        }
                    })
            } else {
                false
            }
        });
        assert!(has_sup, "footnote reference should render as <sup>");
    }

    #[test]
    fn footnote_back_reference_target_exists() {
        // The <li> back-reference links to #fnref-1, so the referencing <a>
        // must carry id="fnref-1" for the anchor to resolve.
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(&ctx, "See note[^1].\n\n[^1]: This is a footnote.")
            .expect("should parse");

        let paragraph = find_element(&view.nodes, "p").expect("should have paragraph");
        let sup = find_element(paragraph.children(), "sup").expect("should have sup");
        let anchor = find_element(sup.children(), "a").expect("should have anchor");
        assert_eq!(
            find_attr_value(anchor, "id"),
            Some("fnref-1".to_string()),
            "reference anchor should carry the back-reference target id"
        );
        assert_eq!(
            find_attr_value(anchor, "href"),
            Some("#fn-1".to_string()),
            "reference anchor should link to the footnote definition"
        );

        let ol = find_element(&view.nodes, "ol").expect("should have footnote section");
        let li = find_element(ol.children(), "li").expect("should have footnote item");
        assert_eq!(
            find_attr_value(li, "id"),
            Some("fn-1".to_string()),
            "footnote item should carry the reference target id"
        );
        let back_ref = find_element(li.children(), "a").expect("should have back-reference");
        assert_eq!(
            find_attr_value(back_ref, "href"),
            Some("#fnref-1".to_string()),
            "back-reference should link to the reference anchor"
        );
    }

    #[test]
    fn footnote_section_at_document_end() {
        // Footnote definitions should render as <ol> at document end.
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(&ctx, "See note[^1].\n\n[^1]: This is a footnote.")
            .expect("should parse");
        // The view should contain an <ol> with footnote items.
        let has_ol = view.nodes.iter().any(|n| {
            if let Node::Element(e) = n {
                e.name().string_name().as_deref() == Some("ol")
            } else {
                false
            }
        });
        assert!(
            has_ol,
            "footnote section should render as <ol> at document end"
        );
    }

    #[test]
    fn footnote_definition_skipped_during_main_walk() {
        // FootnoteDefinition nodes should not appear as rendered content
        // in the main walk (only in the document-end <ol> section).
        let ctx = WalkContext::empty();
        let view = parse_and_walk_full_ctx(&ctx, "Body text.\n\n[^1]: Footnote content.")
            .expect("should parse");
        // Only body paragraph should appear (no footnote content inline).
        // Since no FootnoteReference is used, there should be no <ol> section.
        assert!(
            view.nodes.len() == 1,
            "should only have body paragraph when no footnote reference used, got {}",
            view.nodes.len()
        );
    }
}
