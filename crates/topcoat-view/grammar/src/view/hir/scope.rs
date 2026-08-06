use proc_macro2::TokenStream;
use quote::quote;
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use super::{
    Node, StaticSegment,
    emit::{Emit, Emitter},
};

/// The lowered form of a `view!` invocation: the HIR between the view AST and
/// the emitted `TokenStream`. Built by [`ViewBuilder`](super::ViewBuilder).
pub(crate) struct Scope {
    nodes: Vec<Node>,
}

impl Scope {
    pub(super) fn new(nodes: Vec<Node>) -> Self {
        Self { nodes }
    }

    /// Emits the expansion of a top-level `view!` invocation: the view
    /// expression wrapped in an `async` block.
    pub fn emit(&self) -> TokenStream {
        let view = self.emit_expr();
        quote! { async {
            use #topcoat_view::internal::*;
            ::core::result::Result::<#topcoat_view::View, #topcoat_error::Error>::Ok(#view)
        }.await }
    }

    /// Emits this scope as an expression yielding a
    /// [`View`](topcoat_view::View).
    ///
    /// Nested scopes (control-flow bodies and component children) are emitted
    /// with this and hoisted by the enclosing scope, so their expansion runs
    /// inside the top-level `async` block where the `internal` helpers are in
    /// scope and `?` propagates to the block's `Result`.
    pub(crate) fn emit_expr(&self) -> TokenStream {
        if self.nodes.is_empty() {
            // Optimized path: The view has no content.
            quote! { #topcoat_view::View::empty() }
        } else if self.nodes.len() == 1
            && let Node::StaticSegment(StaticSegment { string }) = &self.nodes[0]
        {
            // Optimized path: The view is a single static string, which needs
            // no instruction block.
            quote! { #topcoat_view::View::unescaped_unchecked(#string) }
        } else {
            let mut emitter = Emitter::new();
            for node in &self.nodes {
                node.emit(&mut emitter);
            }
            emitter.finish()
        }
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;
    use crate::view::hir::{ExprKind, ViewBuilder};

    fn rendered(builder: ViewBuilder) -> String {
        builder.finish().emit().to_string()
    }

    #[test]
    fn empty_top_level_view_emits_view_empty() {
        let out = rendered(ViewBuilder::new());
        assert!(out.contains("async"));
        assert!(out.contains(&quote! { #topcoat_view::View::empty }.to_string()));
    }

    #[test]
    fn adjacent_literal_text_is_concatenated() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<div>");
        builder.text("hello");
        builder.str_unescaped("</div>");
        let out = rendered(builder);
        assert!(out.contains("\"<div>hello</div>\""));
    }

    #[test]
    fn single_static_segment_needs_no_instruction_block() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<div>static</div>");
        let out = rendered(builder);
        assert!(out.contains("unescaped_unchecked"));
        assert!(!out.contains("__build_view"));
    }

    #[test]
    fn literal_text_is_escaped_for_its_position() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p>");
        builder.text("a < b & \"c\"");
        builder.str_unescaped("</p>");
        let out = rendered(builder);
        assert!(out.contains("a &lt; b &amp; \\\"c\\\""));

        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p x=\"");
        builder.attribute_value("a < b & \"c\"");
        builder.str_unescaped("\">");
        let out = rendered(builder);
        assert!(out.contains("a < b &amp; &quot;c&quot;"));
    }

    #[test]
    fn expression_is_hoisted_and_pushed_with_kind_helper() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p>");
        builder.expr(ExprKind::Node, quote! { value });
        builder.str_unescaped("</p>");
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = value"));
        assert!(out.contains("__build_view"));
        assert!(out.contains("__unescaped (__cx , __parts , \"<p>\")"));
        assert!(out.contains("__node (__cx , __parts , __expr0)"));
        assert!(out.contains("__unescaped (__cx , __parts , \"</p>\")"));
    }

    #[test]
    fn if_else_hoists_both_branches_as_views() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.str_unescaped("yes");
            else_branch.str_unescaped("no");
        });
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = if cond"));
        assert!(out.contains("else"));
        assert!(out.contains("\"yes\""));
        assert!(out.contains("\"no\""));
        assert!(out.contains("__view (__cx , __parts , __expr0)"));
    }

    #[test]
    fn if_without_else_falls_back_to_the_empty_view() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.str_unescaped("yes");
        });
        let out = rendered(builder);
        assert!(out.contains("if cond"));
        assert!(out.contains(&quote! { #topcoat_view::View::empty }.to_string()));
    }

    #[test]
    fn for_loop_collects_views_and_splices_them_in_order() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.str_unescaped("x");
        });
        let out = rendered(builder);
        assert!(out.contains("for x in xs"));
        assert!(out.contains("__views . push"));
        assert!(out.contains("for __loop_view in __expr0"));
    }

    #[test]
    fn match_expr_renders_arms_with_optional_guard() {
        let mut builder = ViewBuilder::new();
        builder.match_expr(&syn::parse_quote!(v), |arms| {
            arms.arm(&syn::parse_quote!(A), None, |body| {
                body.str_unescaped("a");
            });
            arms.arm(
                &syn::parse_quote!(B),
                Some(&syn::parse_quote!(flag)),
                |body| {
                    body.str_unescaped("b");
                },
            );
        });
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = match v"));
        assert!(out.contains("A =>"));
        assert!(out.contains("B if flag =>"));
        assert!(out.contains("__view (__cx , __parts , __expr0)"));
    }

    #[test]
    fn local_binding_emits_let_statement() {
        let mut builder = ViewBuilder::new();
        builder.local_binding(&syn::parse_quote!(x), &syn::parse_quote!(value));
        builder.str_unescaped("ok");
        let out = rendered(builder);
        assert!(out.contains("let x = value"));
    }

    #[test]
    fn expr_kind_selects_matching_helper() {
        for (kind, expected) in [
            (ExprKind::Node, "__node"),
            (ExprKind::ElementName, "__element_name"),
            (ExprKind::Attribute, "__attribute"),
            (ExprKind::AttributeUnescaped, "__attribute_unescaped"),
            (ExprKind::AttributeKey, "__attribute_key"),
            (ExprKind::AttributeValue, "__attribute_value"),
            (ExprKind::Attributes, "__attributes"),
        ] {
            let mut builder = ViewBuilder::new();
            builder.expr(kind, quote! { v });
            assert!(
                rendered(builder).contains(expected),
                "expected helper `{expected}`",
            );
        }
    }
}
