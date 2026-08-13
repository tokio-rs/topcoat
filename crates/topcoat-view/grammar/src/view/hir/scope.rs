use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use super::{
    LiveNode, Node, StaticSegment,
    emit::{Emit, EmitMode, Emitter},
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

    pub fn emit_root(&self) -> TokenStream {
        let view = self.emit_view();
        let body = quote! {
            async {
                ::core::result::Result::<#topcoat_view::View, #topcoat_error::Error>::Ok(#view)
            }
        };
        if self.is_static() {
            quote! { #body.await }
        } else {
            quote! { #topcoat_view::internal::build(#body).await }
        }
    }

    /// Whether this scope emits through an optimized static path that never
    /// touches an instruction memory.
    pub(crate) fn is_static(&self) -> bool {
        self.nodes.is_empty()
            || (self.nodes.len() == 1 && matches!(&self.nodes[0], Node::StaticSegment(_)))
    }

    /// Whether this scope renders components or reactive nodes, directly or
    /// anywhere under its nested scopes.
    pub(crate) fn is_async(&self) -> bool {
        self.nodes.iter().any(Node::is_async)
    }

    /// Emits this scope as an expression yielding a
    /// [`View`](topcoat_view::View).
    ///
    /// Nested scopes (control-flow bodies and component children) are emitted
    /// with this and hoisted by the enclosing scope, so their expansion runs
    /// inside the top-level `async` block where `?` propagates to the block's
    /// `Result`.
    ///
    /// When more than one of the scope's nodes renders components, their
    /// futures are joined so sibling components render concurrently. With at
    /// most one such node there is nothing to overlap, and the node awaits
    /// inline.
    pub(crate) fn emit_view(&self) -> TokenStream {
        if let Some(optimized) = self.emit_static() {
            optimized
        } else {
            let concurrent = self.nodes.iter().filter(|node| node.is_async()).count() >= 2;
            let mut emitter = Emitter::new(EmitMode::Blocking, !concurrent);
            for node in &self.nodes {
                node.emit(&mut emitter);
            }
            emitter.finish()
        }
    }

    /// Emits the optimized expression for a scope with no dynamic content,
    /// which needs no instruction block; `None` when the scope has some.
    fn emit_static(&self) -> Option<TokenStream> {
        if self.nodes.is_empty() {
            // Optimized path: The view has no content.
            Some(quote! { #topcoat_view::View::empty() })
        } else if self.nodes.len() == 1
            && let Node::StaticSegment(StaticSegment { string }) = &self.nodes[0]
        {
            // Optimized path: The view is a single static string, which needs
            // no instruction block.
            Some(quote! { #topcoat_view::View::unescaped_unchecked(#string) })
        } else {
            None
        }
    }

    /// Emits this scope as an expression yielding a
    /// [`View`](topcoat_view::View) in a nested position of a live frame.
    ///
    /// The scope's invocations and reactive nodes register with the
    /// enclosing frame's `__refresh`, so the expression is synchronous:
    /// placeholders stand in for everything that resolves later, and the
    /// frame's barrier is the join.
    pub(crate) fn emit_view_live(&self) -> TokenStream {
        if let Some(optimized) = self.emit_static() {
            optimized
        } else {
            let mut emitter = Emitter::new(EmitMode::Live, false);
            for node in &self.nodes {
                node.emit(&mut emitter);
            }
            emitter.finish_nested()
        }
    }

    /// Emits this scope as one live frame: an expression of the frame's
    /// final status that builds the view, delivers it through `delivery`,
    /// and drives the frame's live work to completion.
    ///
    /// The frame's slots and tickets are registered before its `RefreshSet`,
    /// so the set's entries capture them without borrowing locals the set
    /// outlives; `attach` adopts work created before the frame existed, such
    /// as a `view!` bound mid-body.
    pub(crate) fn emit_frame_live(
        &self,
        attach: &TokenStream,
        delivery: &TokenStream,
    ) -> TokenStream {
        if attach.is_empty()
            && let Some(optimized) = self.emit_static()
        {
            // A frame with no dynamic content and nothing attached delivers
            // its static view and is done; no set is needed.
            return quote! {{
                let __view = #optimized;
                #delivery
                ::core::result::Result::<(), #topcoat_error::Error>::Ok(())
            }};
        }
        let preludes = self.live_preludes();
        let view = {
            let mut emitter = Emitter::new(EmitMode::Live, false);
            for node in &self.nodes {
                node.emit(&mut emitter);
            }
            emitter.finish()
        };
        quote! {{
            let __frame = #topcoat_view::internal::new_frame();
            #preludes
            let mut __refresh = #topcoat_view::internal::RefreshSet::with_frame(__frame);
            #attach
            let __view = #view;
            #delivery
            let __done = __refresh.run().await;
            __done
        }}
    }

    /// Returns the statements registering this frame's reactive nodes'
    /// slots and tickets, ahead of the frame's set.
    fn live_preludes(&self) -> TokenStream {
        let mut preludes = TokenStream::new();
        self.collect_live(&mut |node| {
            let view = format_ident!("__node{}", node.ordinal);
            let slot = format_ident!("__node_slot{}", node.ordinal);
            let ticket = format_ident!("__node_ticket{}", node.ordinal);
            preludes.extend(quote! {
                let (#view, #slot) = #topcoat_view::internal::reserve();
                let #ticket = #topcoat_view::internal::new_ticket(__frame);
            });
        });
        preludes
    }

    /// Walks the reactive nodes belonging to this frame: the scope's own and
    /// those of nested scopes emitted inline, stopping at sub-frames (arm
    /// bodies and loop iterations), which register their own.
    fn collect_live(&self, f: &mut impl FnMut(&LiveNode)) {
        for node in &self.nodes {
            match node {
                Node::Live(live) => f(live),
                Node::IfElse(node) => {
                    node.then_branch.collect_live(f);
                    node.else_branch.collect_live(f);
                }
                Node::MatchExpr(node) => {
                    for arm in &node.arms {
                        arm.body.collect_live(f);
                    }
                }
                Node::Component(node) => {
                    if let Some(children) = &node.children {
                        children.collect_live(f);
                    }
                }
                Node::ForLoop(_)
                | Node::StaticSegment(_)
                | Node::ExprNode(_)
                | Node::Local(_)
                | Node::Statement(_) => {}
            }
        }
    }

    /// Emits this scope as an expression yielding a future of the view, for a
    /// position joined with sibling futures in the block the expression is
    /// evaluated in, like the branches of an `if`.
    ///
    /// The future borrows from its environment, so it must not outlive the
    /// enclosing block; for one that does, use
    /// [`emit_owned_future`](Self::emit_owned_future).
    pub(crate) fn emit_future(&self) -> TokenStream {
        self.future(&quote! { async })
    }

    /// Emits this scope as an expression yielding a future of the view that
    /// owns everything it captures, so it can outlive the block the
    /// expression is evaluated in, like one iteration's future outliving the
    /// iteration in a joined `for` loop.
    pub(crate) fn emit_owned_future(&self) -> TokenStream {
        self.future(&quote! { async move })
    }

    fn future(&self, header: &TokenStream) -> TokenStream {
        let view = self.emit_view();
        if self.is_async() {
            quote! {
                #header {
                    ::core::result::Result::<_, #topcoat_error::Error>::Ok(#view)
                }
            }
        } else {
            // Without components the view builds synchronously where the
            // expression is evaluated and only needs wrapping into a ready
            // future.
            quote! { #topcoat_view::internal::ready(#view) }
        }
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::quote;
    use syn::Expr;

    use super::*;
    use crate::view::{
        NamedArg, NamedArgValue, Nodes,
        hir::{ExprKind, ViewBuilder},
    };

    fn rendered(builder: ViewBuilder) -> String {
        builder.finish().emit_root().to_string()
    }

    fn add_component(builder: &mut ViewBuilder, name: &str) {
        add_component_with_children(builder, name, &syn::parse_quote!());
    }

    fn add_component_with_children(builder: &mut ViewBuilder, name: &str, children: &Nodes) {
        let path = syn::parse_str(name).unwrap();
        builder.component(&path, Vec::new(), None, children, Span::call_site());
    }

    fn add_keyed_component(builder: &mut ViewBuilder, name: &str, key: &Expr) {
        let path = syn::parse_str(name).unwrap();
        let key = NamedArg {
            ident: syn::parse_quote!(key),
            colon: syn::token::Colon::default(),
            value: NamedArgValue::Expr(key.clone()),
        };
        builder.component(
            &path,
            Vec::new(),
            Some(&key),
            &syn::parse_quote!(),
            Span::call_site(),
        );
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
        assert!(!out.contains("internal :: block"));
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
    fn expression_is_hoisted_and_pushed_with_builder_method() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p>");
        builder.expr(ExprKind::Node, quote! { value });
        builder.str_unescaped("</p>");
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = value"));
        assert!(out.contains("internal :: block"));
        assert!(out.contains("__b . markup (& \"<p>\")"));
        assert!(out.contains("__b . node (__expr0)"));
        assert!(out.contains("__b . markup (& \"</p>\")"));
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
        assert!(out.contains("__b . view (__expr0)"));
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
        assert!(out.contains("__b . view (__expr0)"));
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
    fn single_component_awaits_inline() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "solo");
        builder.str_unescaped("<hr>");
        let out = rendered(builder);
        assert!(out.contains(". await ?"));
        assert!(!out.contains("try_join !"));
    }

    #[test]
    fn async_children_resolve_through_a_reserved_slot() {
        let mut builder = ViewBuilder::new();
        add_component_with_children(&mut builder, "wrapper", &syn::parse_quote!(inner()));
        let out = rendered(builder);
        assert!(out.contains("reserve ()"));
        assert!(out.contains(". child (__placeholder)"));
        assert!(out.contains("try_join ! (__render , __child)"));
        assert!(out.contains("__slot . fill (__child)"));
    }

    #[test]
    fn sync_children_build_before_the_props() {
        let mut builder = ViewBuilder::new();
        add_component_with_children(&mut builder, "wrapper", &syn::parse_quote!("static"));
        let out = rendered(builder);
        assert!(!out.contains("reserve ()"));
        assert!(!out.contains("try_join !"));
    }

    #[test]
    fn sibling_components_are_joined() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "first");
        add_component(&mut builder, "second");
        let out = rendered(builder);
        assert!(out.contains("try_join ! (__expr0 , __expr1)"));
        assert!(!out.contains(". await ?"));
    }

    #[test]
    fn a_component_derives_its_identity_at_the_invocation_site() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "solo");
        let out = rendered(builder);
        assert!(out.contains("IdentityFuture :: new"));
        assert!(out.contains("SiteKey :: new"));
        assert!(out.contains("file ! ()"));
    }

    #[test]
    fn sites_are_numbered_in_lowering_order_across_nested_scopes() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "first");
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            add_component(then_branch, "second");
        });
        add_component(&mut builder, "third");
        let out = rendered(builder);
        for ordinal in ["0u32", "1u32", "2u32"] {
            assert!(out.contains(ordinal), "expected site ordinal {ordinal}");
        }
    }

    #[test]
    fn a_keyed_component_mixes_the_key_into_its_identity() {
        let mut builder = ViewBuilder::new();
        add_keyed_component(&mut builder, "card", &syn::parse_quote!(item.id));
        let out = rendered(builder);
        assert!(out.contains("IdentityFuture :: keyed"));
        assert!(out.contains("item . id"));
    }

    #[test]
    fn an_unkeyed_component_in_a_for_body_is_ambiguous() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component(body, "card");
        });
        let out = rendered(builder);
        assert!(out.contains("IdentityFuture :: ambiguous"));
        assert!(out.contains("\"`card`\""));
    }

    #[test]
    fn a_keyed_component_in_a_for_body_is_not_ambiguous() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_keyed_component(body, "card", &syn::parse_quote!(x));
        });
        let out = rendered(builder);
        assert!(out.contains("IdentityFuture :: keyed"));
        assert!(!out.contains("IdentityFuture :: ambiguous"));
    }

    #[test]
    fn branches_inside_a_for_body_still_repeat() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.if_else(&syn::parse_quote!(cond), |then_branch, _| {
                add_component(then_branch, "card");
            });
        });
        let out = rendered(builder);
        assert!(out.contains("IdentityFuture :: ambiguous"));
    }

    #[test]
    fn children_derive_below_their_component_instead_of_repeating() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component_with_children(body, "wrapper", &syn::parse_quote!(inner()));
        });
        let out = rendered(builder);
        // The wrapper repeats unkeyed, but its child does not repeat
        // relative to it; the wrapper's ambiguity poisons the child at
        // runtime instead.
        assert_eq!(out.matches("IdentityFuture :: ambiguous").count(), 1);
        assert_eq!(out.matches("IdentityFuture :: new").count(), 1);
    }

    #[test]
    fn for_loop_with_component_body_joins_iterations() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component(body, "item");
        });
        let out = rendered(builder);
        assert!(out.contains("try_join_all"));
        // The iteration future owns its bindings, so it outlives the
        // iteration; the single loop node then awaits inline.
        assert!(out.contains("async move"));
        assert!(out.contains(". await ?"));
    }

    #[test]
    fn if_else_in_a_joined_position_wraps_branches_in_either() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "sibling");
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            add_component(then_branch, "conditional");
        });
        let out = rendered(builder);
        assert!(out.contains("Either :: Left"));
        assert!(out.contains("Either :: Right"));
        assert!(out.contains("try_join ! (__expr0 , __expr1)"));
    }

    #[test]
    fn if_else_without_a_sibling_component_awaits_inline() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            add_component(then_branch, "conditional");
        });
        let out = rendered(builder);
        assert!(out.contains(". await ?"));
        assert!(!out.contains("Either ::"));
        assert!(!out.contains("try_join !"));
    }

    #[test]
    fn match_arms_in_a_joined_position_nest_eithers() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "sibling");
        builder.match_expr(&syn::parse_quote!(v), |arms| {
            arms.arm(&syn::parse_quote!(A), None, |body| {
                add_component(body, "a");
            });
            arms.arm(&syn::parse_quote!(B), None, |body| {
                add_component(body, "b");
            });
            arms.arm(&syn::parse_quote!(C), None, |body| {
                add_component(body, "c");
            });
        });
        let out = rendered(builder);
        let either = quote! { #topcoat_view::internal::Either }.to_string();
        assert!(out.contains(&format!("{either} :: Left")));
        assert!(out.contains(&format!("{either} :: Right ({either} :: Left")));
        assert!(out.contains(&format!("{either} :: Right ({either} :: Right")));
    }

    #[test]
    fn branch_without_components_wraps_into_a_ready_future() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "sibling");
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            add_component(then_branch, "conditional");
        });
        let out = rendered(builder);
        // The empty else branch becomes a ready future so both branches
        // unify into `Either`.
        assert!(out.contains("internal :: ready"));
    }

    #[test]
    fn expr_kind_selects_matching_builder_method() {
        for (kind, method) in [
            (ExprKind::Node, "node"),
            (ExprKind::ElementName, "element_name"),
            (ExprKind::Attribute, "attribute"),
            (ExprKind::AttributeUnescaped, "attribute_unescaped"),
            (ExprKind::AttributeKey, "attribute_key"),
            (ExprKind::AttributeValue, "attribute_value"),
            (ExprKind::Attributes, "attributes"),
        ] {
            let mut builder = ViewBuilder::new();
            builder.expr(kind, quote! { v });
            let expected = format!("__b . {method} (__expr0)");
            assert!(
                rendered(builder).contains(&expected),
                "expected builder call `{expected}`",
            );
        }
    }
}
