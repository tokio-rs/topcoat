use proc_macro2::TokenStream;
use quote::quote;
use topcoat_core_grammar::paths::topcoat_view;

use super::{
    Bindings, Node,
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

    /// Emits a top-level `view!` invocation: a `MoveView` whose `async move`
    /// body builds the scope's view and drives it in place.
    ///
    /// The block captures every value the template uses, so the view owns
    /// its data and the expressions inside borrow from the block. The built
    /// view never leaves the block, which keeps those borrows valid.
    pub fn emit_root(&self) -> TokenStream {
        self.emit_move_view(quote! { move }, TokenStream::new())
    }

    /// Emits this scope as the body of a branch or iteration whose pattern
    /// binds `bindings`.
    ///
    /// The bound values die with the branch or iteration that produced them
    /// while the view lives on, so the view must own them: a nested
    /// `MoveView` carries them in a `Capture` packed where they are still
    /// alive and taken back apart inside its body. The body is not `move`,
    /// so everything else stays borrowed from the enclosing scope and a
    /// value shared by all iterations is not moved into the first. Without
    /// bindings the scope is a plain view in the enclosing scope.
    pub(crate) fn emit_captured(&self, bindings: &Bindings) -> TokenStream {
        if bindings.is_empty() {
            return self.emit_view();
        }
        let idents = bindings.idents();
        let rebinds = bindings.rebinds();
        let view = self.emit_move_view(
            TokenStream::new(),
            quote! { let (#(#rebinds,)*) = __captured.take(); },
        );
        quote! {{
            let __captured = #topcoat_view::internal::Capture((#(#idents,)*));
            #view
        }}
    }

    /// Emits this scope as a `MoveView` whose async body runs `prologue`,
    /// builds the scope's view, and drives it in place.
    fn emit_move_view(&self, move_token: TokenStream, prologue: TokenStream) -> TokenStream {
        let view = self.emit_view();
        quote! {
            #topcoat_view::internal::MoveView::new(async #move_token {
                #prologue
                let __view = #view;
                <#topcoat_view::internal::MoveView>::drive(__cx, __view).await
            })
        }
    }

    /// Emits this scope as an inert view value: a block expression that
    /// evaluates the scope's expressions in source order and builds its
    /// `JoinView`.
    ///
    /// The view owns the evaluated values; whatever the expressions borrow
    /// from the environment, it borrows. A nested scope built inside a
    /// branch or iteration takes its pattern bindings with it, since the
    /// expressions move them into the view.
    pub(crate) fn emit_view(&self) -> TokenStream {
        let mut emitter = Emitter::new();
        for node in &self.nodes {
            node.emit(&mut emitter);
        }
        emitter.finish()
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
    fn a_root_view_is_a_move_view_driving_its_body() {
        let out = rendered(ViewBuilder::new());
        assert!(out.contains("MoveView :: new (async move"), "{out}");
        assert!(
            out.contains("MoveView > :: drive (__cx , __view) . await"),
            "{out}"
        );
    }

    #[test]
    fn a_view_without_units_skips_the_join() {
        let out = rendered(ViewBuilder::new());
        assert!(!out.contains("Join :: new"));
        assert!(!out.contains("forward"));
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
    fn static_markup_is_pushed_verbatim() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<div>static</div>");
        let out = rendered(builder);
        assert!(out.contains("__b . markup (& \"<div>static</div>\")"));
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
    fn node_expression_is_hoisted_and_joined_as_a_unit() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p>");
        builder.expr(ExprKind::Node, quote! { value });
        builder.str_unescaped("</p>");
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = value"));
        assert!(out.contains("unit_future (__expr0 , __cx)"));
        assert!(out.contains("Join :: new"));
        assert!(out.contains("__join . first () . await ?"));
        assert!(out.contains("internal :: block"));
        assert!(out.contains("__b . markup (& \"<p>\")"));
        assert!(out.contains("Some (__unit_view) = __view0"));
        assert!(out.contains("__b . view (__unit_view)"));
        assert!(out.contains("__b . markup (& \"</p>\")"));
        assert!(out.contains("forward (__join)"));
    }

    #[test]
    fn if_else_wraps_the_branch_streams_in_either() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.str_unescaped("yes");
            else_branch.str_unescaped("no");
        });
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = if cond"));
        assert!(out.contains("Either :: Left"));
        assert!(out.contains("Either :: Right"));
        assert!(out.contains("\"yes\""));
        assert!(out.contains("\"no\""));
        assert!(out.contains("unit_future (__expr0 , __cx)"));
    }

    #[test]
    fn if_without_else_still_emits_an_else_stream() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.str_unescaped("yes");
        });
        let out = rendered(builder);
        assert!(out.contains("if cond"));
        assert!(out.contains("Either :: Right"));
    }

    #[test]
    fn for_loop_joins_iterations_into_a_loop_view() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.str_unescaped("x");
        });
        let out = rendered(builder);
        assert!(out.contains("for x in xs"));
        assert!(out.contains("__iterations . push"));
        assert!(out.contains("Box :: pin"));
        assert!(out.contains("LoopView :: new (__iterations)"));
        assert!(out.contains("unit_future (__expr0 , __cx)"));
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
        assert!(out.contains("unit_future (__expr0 , __cx)"));
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
    fn a_component_render_becomes_a_joined_unit() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "solo");
        builder.str_unescaped("<hr>");
        let out = rendered(builder);
        assert!(out.contains("Render :: new"));
        assert!(out.contains("unit_future (__expr0 , __cx)"));
    }

    #[test]
    fn children_pass_as_a_lazy_child_value() {
        let mut builder = ViewBuilder::new();
        add_component_with_children(&mut builder, "wrapper", &syn::parse_quote!(inner()));
        let out = rendered(builder);
        assert!(out.contains(". child ("));
        assert!(out.contains("Child :: new"));
        // The child's own stream is passed unpolled: the wrapper decides
        // where, and whether, it is driven.
        assert!(!out.contains("reserve ()"));
        assert!(!out.contains("try_join"));
    }

    #[test]
    fn sibling_components_are_joined() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "first");
        add_component(&mut builder, "second");
        let out = rendered(builder);
        assert!(out.contains("unit_future (__expr0 , __cx)"));
        assert!(out.contains("unit_future (__expr1 , __cx)"));
        assert!(out.contains("Unit :: new (__unit0)"));
        assert!(out.contains("Unit :: new (__unit1)"));
        assert!(out.contains("(__view0 , __view1 ,) = __join . first () . await ?"));
    }

    #[test]
    fn a_component_derives_its_identity_at_the_invocation_site() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "solo");
        let out = rendered(builder);
        assert!(out.contains("IdentityFuture :: child"));
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
        assert_eq!(out.matches("IdentityFuture :: child").count(), 1);
    }

    #[test]
    fn for_loop_with_component_body_boxes_each_iteration() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component(body, "item");
        });
        let out = rendered(builder);
        assert!(out.contains("Box :: pin"));
        assert!(out.contains("LoopView :: new"));
        assert!(out.contains("Render :: new"));
    }

    #[test]
    fn a_for_loop_body_borrows_everything_but_its_bindings() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component(body, "item");
        });
        let out = rendered(builder);
        // Only the root stream owns its captures; the iteration streams
        // borrow their environment, apart from the captured bindings.
        assert_eq!(out.matches("async move").count(), 1);
        assert!(out.contains("Capture ((x ,))"));
        assert!(out.contains("let (x ,) = __captured . take ()"));
    }

    #[test]
    fn an_if_let_branch_captures_its_bindings() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "sibling");
        builder.if_else(
            &syn::parse_quote!(let Some(status) = value),
            |then_branch, _| {
                add_component(then_branch, "conditional");
            },
        );
        let out = rendered(builder);
        assert!(out.contains("Capture ((status ,))"));
        assert!(out.contains("let (status ,) = __captured . take ()"));
    }

    #[test]
    fn a_branch_without_bindings_emits_no_capture() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "sibling");
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            add_component(then_branch, "conditional");
        });
        let out = rendered(builder);
        assert!(!out.contains("Capture"));
    }

    #[test]
    fn match_arms_in_a_joined_position_capture_their_own_bindings() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "sibling");
        builder.match_expr(&syn::parse_quote!(v), |arms| {
            arms.arm(&syn::parse_quote!(Some(status)), None, |body| {
                add_component(body, "a");
            });
            arms.arm(&syn::parse_quote!(None), None, |body| {
                add_component(body, "b");
            });
        });
        let out = rendered(builder);
        assert!(out.contains("Capture ((status ,))"));
        // The binding-free arm needs no capture.
        assert_eq!(out.matches("Capture (").count(), 1);
    }

    #[test]
    fn match_arms_nest_eithers() {
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
    fn expr_kind_selects_matching_builder_method() {
        for (kind, method) in [
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
