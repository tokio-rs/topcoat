//! Parser configuration for `markdown-rs`.
//!
//! Provides `get_parse_options()` which returns a `ParseOptions` value with
//! GFM, MDX JSX, and frontmatter constructs enabled. HTML passthrough
//! (`html_flow`, `html_text`) is intentionally disabled so that `<` tokens
//! are dispatched to MDX JSX instead of raw HTML productions.

use markdown::{Constructs, ParseOptions};

/// Returns the default parse options for MDX compilation.
///
/// Enables GFM extensions (tables, strikethrough, task lists, autolinks),
/// MDX JSX flow and text support, and YAML frontmatter.
///
/// HTML passthrough is disabled: `markdown-rs` dispatches `<` to HTML flow
/// before MDX JSX, so self-closing tags like `<Widget />` are consumed as
/// COMPLETE HTML productions and silently dropped. Setting `html_flow = false`
/// and `html_text = false` forces all `<` tokens through the MDX JSX path,
/// matching the behavior of `mdxjs-rs` and the MDX spec.
#[must_use]
pub fn get_parse_options() -> ParseOptions {
    let mut constructs = Constructs::gfm();
    constructs.mdx_jsx_flow = true;
    constructs.mdx_jsx_text = true;
    constructs.frontmatter = true;
    // Disable HTML passthrough so that <Widget /> is not consumed as raw
    // COMPLETE HTML. This is a security improvement: raw <script>/<iframe>
    // cannot slip through the MDX pipeline.
    constructs.html_flow = false;
    constructs.html_text = false;
    ParseOptions {
        constructs,
        ..ParseOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_options_enable_gfm_table() {
        let opts = get_parse_options();
        assert!(opts.constructs.gfm_table);
    }

    #[test]
    fn parse_options_enable_gfm_strikethrough() {
        let opts = get_parse_options();
        assert!(opts.constructs.gfm_strikethrough);
    }

    #[test]
    fn parse_options_enable_mdx_jsx() {
        let opts = get_parse_options();
        assert!(opts.constructs.mdx_jsx_flow);
        assert!(opts.constructs.mdx_jsx_text);
    }

    #[test]
    fn parse_options_enable_frontmatter() {
        let opts = get_parse_options();
        assert!(opts.constructs.frontmatter);
    }
}
