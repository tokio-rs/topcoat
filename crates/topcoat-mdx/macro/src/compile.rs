use std::path::Path;

use proc_macro2::Span;
use syn::Path as SynPath;
use topcoat_mdx_grammar::{
    parse::get_parse_options,
    walker::{
        FrontmatterFormat, collect_definitions, extract_frontmatter, node::walk_footnote_section,
        walk_to_writer,
    },
};
use topcoat_view_grammar::view::{ViewWriter, WriteView};

// ---------------------------------------------------------------------------
// Common compile logic shared by compile_mdx! and mdx_page!
// ---------------------------------------------------------------------------

/// Result of compiling an MDX file: the view tokens and how to wrap them.
///
/// Frontmatter is parsed during the walk so that it does not render as body
/// content. It is carried here as written, for macros that deserialize it into
/// a type the caller named.
pub(crate) struct CompiledMdxResult {
    /// The page's frontmatter as written, and the syntax it used.
    pub(crate) frontmatter: Option<(String, FrontmatterFormat)>,
    /// View tokens from the walker. When a wrapper was requested, these are
    /// produced by `ViewWriter::new_nested()` (plain View expression, no
    /// async wrapper).
    pub(crate) view_tokens: proc_macro2::TokenStream,
    /// Whether a wrapper component was requested.
    pub(crate) has_wrapper: bool,
    /// The wrapper component path (set when `has_wrapper` is true).
    pub(crate) wrapper_path: Option<SynPath>,
}

/// Shared inner logic: parse markdown content, extract frontmatter, walk mdast.
///
/// Used by both [`compile_mdx_file`] (`compile_mdx!`, `mdx_page!`) and
/// [`generate_page_registration`] (`mdx_pages!`). The `label` parameter controls
/// the prefix in error messages. The `overrides` parameter registers HTML
/// element-to-component substitutions (e.g., `"a" => custom_link`). When
/// `wrapper` is `Some`, uses `ViewWriter::new_nested()` so the output tokens
/// are suitable for a component `child:` prop.
pub(crate) fn parse_and_walk_mdx(
    components: &[(String, SynPath)],
    overrides: &[(&'static str, SynPath)],
    wrapper: Option<&SynPath>,
    content: &str,
    label: &str,
    span: Span,
) -> Result<CompiledMdxResult, syn::Error> {
    // Parse with markdown-rs.
    let options = get_parse_options();
    let root = markdown::to_mdast(content, &options)
        .map_err(|e| syn::Error::new(span, format!("{label} parse error: {e}")))?;

    // Extract frontmatter from root node.
    let frontmatter_content = extract_frontmatter(&root);

    // Build override registry from the borrowed slice into owned storage
    // that lives for the WalkContext lifetime.
    let owned_overrides: Vec<(&'static str, SynPath)> = overrides
        .iter()
        .map(|(tag, path)| (*tag, path.clone()))
        .collect();

    // First pass: collect link/image definitions and footnote definitions, so
    // that reference nodes encountered during the walk can resolve against them.
    let (definitions, footnotes) = match root {
        markdown::mdast::Node::Root(ref r) => collect_definitions(r),
        _ => Default::default(),
    };
    let ctx = topcoat_mdx_grammar::walker::WalkContext::with_maps(
        components,
        &owned_overrides,
        span,
        definitions,
        footnotes,
    );

    // Root children, skipping the frontmatter node when present.
    let post_fm_children: &[markdown::mdast::Node] =
        if let markdown::mdast::Node::Root(ref r) = root {
            let start_idx = usize::from(frontmatter_content.is_some());
            &r.children[start_idx..]
        } else {
            &[]
        };

    // Walk mdast into a ViewWriter, skipping the frontmatter node.
    // Use new_nested() when a wrapper is specified so the tokens are suitable
    // for a component `child:` prop (no async wrapper).
    let mut writer = if wrapper.is_some() {
        ViewWriter::new_nested()
    } else {
        ViewWriter::new()
    };

    for child in post_fm_children {
        walk_to_writer(&ctx, child, &mut writer);
    }

    // Second pass: footnote definitions render as a numbered section at the end
    // of the document, in first-reference order.
    let footnote_order = ctx.footnote_order.borrow().clone();
    if !footnote_order.is_empty() {
        walk_footnote_section(&ctx, &footnote_order).write(&mut writer);
    }

    // Drain walker error buffer into syn::Error diagnostics.
    let errors: Vec<String> = ctx.errors.borrow_mut().drain(..).collect();
    if !errors.is_empty() {
        let mut combined_err = syn::Error::new(span, errors[0].clone());
        for err in &errors[1..] {
            combined_err.combine(syn::Error::new(span, err.clone()));
        }
        return Err(combined_err);
    }

    let inner_tokens = writer.into_token_stream();

    Ok(CompiledMdxResult {
        frontmatter: frontmatter_content,
        view_tokens: inner_tokens,
        has_wrapper: wrapper.is_some(),
        wrapper_path: wrapper.cloned(),
    })
}

/// Shared logic: resolve path, read file, parse, extract frontmatter, walk.
/// When `wrapper` is `Some`, emits a component invocation wrapping the view tokens.
pub(crate) fn compile_mdx_file(
    components: &[(String, SynPath)],
    overrides: &[(&'static str, SynPath)],
    wrapper: Option<&SynPath>,
    path_str: &str,
    span: Span,
) -> Result<CompiledMdxResult, syn::Error> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_owned());
    let resolved = Path::new(&manifest_dir).join(path_str);

    // Security: verify resolved path stays within manifest directory.
    let canonical = resolved.canonicalize().map_err(|e| {
        syn::Error::new(
            span,
            format!("compile_mdx! cannot resolve path '{path_str}': {e}"),
        )
    })?;
    let canonical_manifest = std::path::Path::new(&manifest_dir)
        .canonicalize()
        .map_err(|e| {
            syn::Error::new(
                span,
                format!(
                    "compile_mdx! cannot canonicalize CARGO_MANIFEST_DIR '{manifest_dir}': {e}"
                ),
            )
        })?;

    if !canonical.starts_with(&canonical_manifest) {
        return Err(syn::Error::new(
            span,
            format!("compile_mdx! path '{path_str}' escapes CARGO_MANIFEST_DIR"),
        ));
    }

    let content = std::fs::read_to_string(&canonical).map_err(|e| {
        syn::Error::new(span, format!("compile_mdx! cannot read '{path_str}': {e}"))
    })?;

    parse_and_walk_mdx(
        components,
        overrides,
        wrapper,
        &content,
        "compile_mdx!",
        span,
    )
}
