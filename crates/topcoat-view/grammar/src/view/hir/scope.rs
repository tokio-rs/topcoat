use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use super::{ExprKind, Node};

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
        let view = self.emit_view();
        quote! { async { ::core::result::Result::<#topcoat_view::View, #topcoat_error::Error>::Ok(#view) }.await }
    }

    /// Emits a nested view (e.g. a component's children), which is spliced
    /// into the parent's `async` block and must not introduce its own.
    pub fn emit_nested(&self) -> TokenStream {
        self.emit_view()
    }

    fn emit_view(&self) -> TokenStream {
        if self.nodes.is_empty() {
            // Optimized path: The view has no content.
            quote! { #topcoat_view::View::empty() }
        } else if self.nodes.len() == 1
            && let Node::Static { string } = &self.nodes[0]
        {
            quote! { #topcoat_view::View::unescaped_unchecked(#string) }
        } else {
            let statements = Self::emit_nodes(&self.nodes);
            quote! {{
                use #topcoat_view::internal::*;
                let mut __parts = #topcoat_view::ViewParts::new();
                #statements
                #topcoat_view::View::new(__parts)
            }}
        }
    }

    fn emit_nodes(nodes: &[Node]) -> TokenStream {
        let mut output = TokenStream::new();
        for node in nodes {
            match node {
                Node::Static { string } => {
                    let helper = ExprKind::Unescaped.helper();
                    let tokens = quote! { #string };
                    quote! { #helper(__cx, &mut __parts, #tokens); }
                }
                Node::Expr { kind, tokens } => {
                    let helper = kind.helper();
                    quote! { #helper(__cx, &mut __parts, #tokens); }
                }
                Node::Local { pat, expr } => {
                    quote! { let #pat = #expr; }
                }
                Node::Statement { tokens } => {
                    quote! { #tokens }
                }
                Node::If {
                    expr,
                    then_branch,
                    else_branch,
                } => {
                    let then_tokens = Self::emit_nodes(&then_branch.nodes);
                    let else_tokens = (!else_branch.nodes.is_empty()).then(|| {
                        let tokens = Self::emit_nodes(&else_branch.nodes);
                        quote! { else { #tokens } }
                    });
                    quote! {
                        if #expr {
                            #then_tokens
                        }
                        #else_tokens
                    }
                }
                Node::For { pat, expr, body } => {
                    let body = Self::emit_nodes(&body.nodes);
                    quote! {
                        for #pat in #expr {
                            #body
                        }
                    }
                }
                Node::Match { expr, arms } => {
                    let arm_tokens = arms.iter().map(|arm| {
                        let pat = &arm.pat;
                        let guard = arm.guard.as_ref().map(|g| quote! { if #g });
                        let body = Self::emit_nodes(&arm.body.nodes);
                        quote! {
                            #pat #guard => { #body }
                        }
                    });
                    quote! {
                        match #expr {
                            #(#arm_tokens,)*
                        }
                    }
                }
            }
            .to_tokens(&mut output);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;
    use crate::view::hir::ViewBuilder;

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
    fn empty_nested_view_omits_async_wrapper() {
        // Nested views (e.g. component children) are spliced into a parent and
        // must not introduce their own async block.
        let out = ViewBuilder::new().finish().emit_nested().to_string();
        assert!(!out.contains("async"));
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
    fn expression_breaks_static_segment_with_kind_helper() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p>");
        builder.expr(ExprKind::Node, quote! { value });
        builder.str_unescaped("</p>");
        let out = rendered(builder);
        assert!(out.contains("__unescaped (__cx , & mut __parts , \"<p>\")"));
        assert!(out.contains("__node (__cx , & mut __parts , value)"));
        assert!(out.contains("__unescaped (__cx , & mut __parts , \"</p>\")"));
    }

    #[test]
    fn if_else_renders_both_branches() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.str_unescaped("yes");
            else_branch.str_unescaped("no");
        });
        let out = rendered(builder);
        assert!(out.contains("if cond"));
        assert!(out.contains("else"));
        assert!(out.contains("\"yes\""));
        assert!(out.contains("\"no\""));
    }

    #[test]
    fn if_without_else_omits_else_branch() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.str_unescaped("yes");
        });
        let out = rendered(builder);
        assert!(out.contains("if cond"));
        assert!(!out.contains("else"));
    }

    #[test]
    fn for_loop_wraps_body_in_for_in_expr() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.str_unescaped("x");
        });
        let out = rendered(builder);
        assert!(out.contains("for x in xs"));
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
        assert!(out.contains("match v"));
        assert!(out.contains("A =>"));
        assert!(out.contains("B if flag =>"));
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
            (ExprKind::Unescaped, "__unescaped"),
            (ExprKind::Node, "__node"),
            (ExprKind::View, "__view"),
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
