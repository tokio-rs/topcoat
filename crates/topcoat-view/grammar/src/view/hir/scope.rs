use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use super::{
    ExprKind, ExprNode, ForLoop, IfElse, Local, MatchExpr, Node, Statement, StaticSegment,
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
        let future = self.emit_future();
        quote! { (#future).await }
    }

    pub(super) fn emit_future(&self) -> TokenStream {
        if self.contains_component() {
            self.emit_tree_future()
        } else {
            let view = self.emit_ready_view();
            quote! {
                async {
                    ::core::result::Result::<
                        #topcoat_view::View,
                        #topcoat_error::Error,
                    >::Ok(#view)
                }
            }
        }
    }

    fn emit_ready_view(&self) -> TokenStream {
        if self.nodes.is_empty() {
            // Optimized path: The view has no content.
            Self::emit_empty_view()
        } else if self.nodes.len() == 1
            && let Node::StaticSegment(StaticSegment { string }) = &self.nodes[0]
        {
            Self::emit_static_view(string)
        } else {
            let statements = Self::emit_ready_nodes(&self.nodes);
            quote! {{
                use #topcoat_view::internal::*;
                let mut __parts = #topcoat_view::ViewParts::new();
                #statements
                #topcoat_view::View::new(__parts)
            }}
        }
    }

    pub(super) fn contains_component(&self) -> bool {
        self.nodes.iter().any(Node::contains_component)
    }

    fn emit_tree_future(&self) -> TokenStream {
        Self::emit_tree_future_for(&self.nodes)
    }

    fn emit_tree_future_for(nodes: &[Node]) -> TokenStream {
        // Leading locals live outside the tree so every pending node can
        // borrow them. Later locals begin a nested future for the remainder.
        let prologue_len = nodes
            .iter()
            .take_while(|node| matches!(node, Node::Local(_) | Node::Statement(_)))
            .count();
        let prologue_nodes = &nodes[..prologue_len];
        let prologue = Self::emit_ready_nodes(prologue_nodes);
        let prologue_view = prologue_nodes
            .iter()
            .any(|node| matches!(node, Node::Statement(_)))
            .then(|| {
                quote! {
                    __tree.push_view(#topcoat_view::View::new(
                        ::core::mem::take(&mut __parts),
                    ));
                }
            });
        let (statements, parts_dirty) = Self::emit_tree_nodes(&nodes[prologue_len..]);
        let flush = parts_dirty.then(|| {
            quote! {
                __tree.push_view(#topcoat_view::View::new(__parts));
            }
        });

        quote! {
            async {
                use #topcoat_view::internal::*;
                let mut __parts = #topcoat_view::ViewParts::new();
                #prologue
                let mut __tree = __ViewTree::new();
                #prologue_view
                #statements
                #flush
                __tree.resolve().await
            }
        }
    }

    fn emit_tree_nodes(nodes: &[Node]) -> (TokenStream, bool) {
        let mut output = TokenStream::new();
        let mut parts_dirty = false;

        for (index, node) in nodes.iter().enumerate() {
            match node {
                Node::StaticSegment(StaticSegment { string }) => {
                    parts_dirty = true;
                    let helper = ExprKind::Unescaped.helper();
                    quote! { #helper(__cx, &mut __parts, #string); }
                }
                Node::ExprNode(ExprNode { kind, tokens }) => {
                    parts_dirty = true;
                    let helper = kind.helper();
                    quote! { #helper(__cx, &mut __parts, #tokens); }
                }
                Node::Component(component) => {
                    let flush = Self::flush_tree_parts(&mut parts_dirty);
                    let future = component.emit_future();
                    quote! {
                        #flush
                        __tree.push_future(#future);
                    }
                }
                Node::Local(_) | Node::Statement(_) => {
                    let flush = Self::flush_tree_parts(&mut parts_dirty);
                    let remainder = Self::emit_tree_future_for(&nodes[index..]);
                    quote! {
                        #flush
                        __tree.push_future(#remainder);
                    }
                    .to_tokens(&mut output);
                    return (output, false);
                }
                Node::IfElse(IfElse {
                    expr,
                    then_branch,
                    else_branch,
                }) if then_branch.contains_component() || else_branch.contains_component() => {
                    let then_branch = then_branch.emit_future();
                    let else_branch = else_branch.emit_future();
                    let flush = Self::flush_tree_parts(&mut parts_dirty);
                    quote! {
                        #flush
                        __tree.push_future(async {
                            if #expr {
                                (#then_branch).await
                            } else {
                                (#else_branch).await
                            }
                        });
                    }
                }
                Node::ForLoop(ForLoop { pat, expr, body }) if body.contains_component() => {
                    let body = body.emit_future();
                    let flush = Self::flush_tree_parts(&mut parts_dirty);
                    quote! {
                        #flush
                        __tree.push_future(async {
                            let __iterations = (#expr)
                                .into_iter()
                                .map(async |#pat| (#body).await);
                            let mut __iteration_parts = #topcoat_view::ViewParts::new();
                            for __iteration in __join_all(__iterations).await {
                                __view(__cx, &mut __iteration_parts, __iteration?);
                            }
                            ::core::result::Result::<
                                #topcoat_view::View,
                                #topcoat_error::Error,
                            >::Ok(#topcoat_view::View::new(__iteration_parts))
                        });
                    }
                }
                Node::MatchExpr(MatchExpr { expr, arms })
                    if arms.iter().any(|arm| arm.body.contains_component()) =>
                {
                    let arm_tokens = arms.iter().map(|arm| {
                        let pat = &arm.pat;
                        let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                        let body = arm.body.emit_future();
                        quote! { #pat #guard => (#body).await }
                    });
                    let flush = Self::flush_tree_parts(&mut parts_dirty);
                    quote! {
                        #flush
                        __tree.push_future(async {
                            match #expr {
                                #(#arm_tokens,)*
                            }
                        });
                    }
                }
                Node::IfElse(_) | Node::ForLoop(_) | Node::MatchExpr(_) => {
                    parts_dirty = true;
                    Self::emit_ready_nodes(core::slice::from_ref(node))
                }
            }
            .to_tokens(&mut output);
        }

        (output, parts_dirty)
    }

    fn flush_tree_parts(parts_dirty: &mut bool) -> TokenStream {
        if *parts_dirty {
            *parts_dirty = false;
            quote! {
                __tree.push_view(#topcoat_view::View::new(
                    ::core::mem::take(&mut __parts),
                ));
            }
        } else {
            TokenStream::new()
        }
    }

    fn emit_ready_nodes(nodes: &[Node]) -> TokenStream {
        let mut output = TokenStream::new();
        for node in nodes {
            match node {
                Node::StaticSegment(StaticSegment { string }) => {
                    let helper = ExprKind::Unescaped.helper();
                    let tokens = quote! { #string };
                    quote! { #helper(__cx, &mut __parts, #tokens); }
                }
                Node::ExprNode(ExprNode { kind, tokens }) => {
                    let helper = kind.helper();
                    quote! { #helper(__cx, &mut __parts, #tokens); }
                }
                Node::Component(_) => unreachable!(),
                Node::Local(Local { pat, expr }) => {
                    quote! { let #pat = #expr; }
                }
                Node::Statement(Statement { tokens }) => {
                    quote! { #tokens }
                }
                Node::IfElse(IfElse {
                    expr,
                    then_branch,
                    else_branch,
                }) => {
                    let then_tokens = Self::emit_ready_nodes(&then_branch.nodes);
                    let else_tokens = (!else_branch.nodes.is_empty()).then(|| {
                        let tokens = Self::emit_ready_nodes(&else_branch.nodes);
                        quote! { else { #tokens } }
                    });
                    quote! {
                        if #expr {
                            #then_tokens
                        }
                        #else_tokens
                    }
                }
                Node::ForLoop(ForLoop { pat, expr, body }) => {
                    let body = Self::emit_ready_nodes(&body.nodes);
                    quote! {
                        for #pat in #expr {
                            #body
                        }
                    }
                }
                Node::MatchExpr(MatchExpr { expr, arms }) => {
                    let arm_tokens = arms.iter().map(|arm| {
                        let pat = &arm.pat;
                        let guard = arm.guard.as_ref().map(|g| quote! { if #g });
                        let body = Self::emit_ready_nodes(&arm.body.nodes);
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

    fn emit_empty_view() -> TokenStream {
        quote! { #topcoat_view::View::empty() }
    }

    fn emit_static_view(s: &str) -> TokenStream {
        quote! { #topcoat_view::View::unescaped_unchecked(#s) }
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
