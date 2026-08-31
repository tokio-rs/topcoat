use proc_macro2::TokenStream;
use quote::quote;
use topcoat_core_grammar::paths::topcoat_view;

use super::{
    Bindings, Node, StaticSegment,
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

    /// Whether this scope renders a component or fills a node position,
    /// directly or anywhere under its nested scopes, so its content can only
    /// resolve by being polled.
    pub(crate) fn is_async(&self) -> bool {
        self.nodes.iter().any(Node::is_async)
    }

    /// Emits this scope as an expression building its instruction block
    /// right where it is evaluated, yielding the handle to the block.
    ///
    /// For a scope that is not async: nothing is polled, and the block may
    /// read whatever the iteration or branch around it binds. A scope
    /// without content, or with literal markup only, needs no block at all.
    pub(crate) fn emit_block(&self) -> TokenStream {
        debug_assert!(!self.is_async(), "an async scope resolves by being polled");
        match self.nodes.as_slice() {
            [] => quote! { #topcoat_view::ViewHandle::empty() },
            [Node::StaticSegment(StaticSegment { string })] => {
                quote! { #topcoat_view::ViewHandle::unescaped_unchecked(#string) }
            }
            nodes => {
                let mut emitter = Emitter::new(true);
                for node in nodes {
                    node.emit(&mut emitter);
                }
                emitter.finish_block()
            }
        }
    }

    /// Emits a top-level `view!` invocation: a `ScopeView` around a
    /// `MoveView` whose `async move` body builds the scope's view and
    /// drives it in place.
    ///
    /// The block captures every value the template uses, so the view owns
    /// its data and the expressions inside borrow from the block. The built
    /// view is driven inside the block that evaluates the template, so what
    /// its expressions borrow from that block is still alive. The
    /// `ScopeView` makes the view own the buffer of the build when it is the
    /// outermost view; nested inside another build, it appends to that
    /// build's buffer.
    ///
    /// With `owns_cx`, the block expects an owned `__cx` context in scope
    /// and captures it, rebinding `__cx` to a borrow of it inside; the view
    /// then does not borrow the caller's context. With `self_contained`,
    /// the view always owns a buffer of its own, so its content renders
    /// anywhere even when it is built inside another build.
    pub fn emit_root(&self, owns_cx: bool, self_contained: bool) -> TokenStream {
        let mut prologue = TokenStream::new();
        if owns_cx {
            prologue.extend(quote! { let __cx = &__cx; });
        }
        let view = self.emit_move_view(&quote! { move }, &prologue);
        if self_contained {
            quote! { #topcoat_view::internal::ScopeView::self_contained(#view) }
        } else {
            quote! { #topcoat_view::internal::ScopeView::new(#view) }
        }
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
            &TokenStream::new(),
            &quote! { let (#(#rebinds,)*) = __captured.take(); },
        );
        quote! {{
            let __captured = #topcoat_view::internal::Capture((#(#idents,)*));
            #view
        }}
    }

    /// Emits this scope as a `MoveView` whose async body runs `prologue`,
    /// builds the scope's view, and drives it in place.
    ///
    /// The drive happens inside the block that evaluates the template, so
    /// the view may borrow from that block's bindings, including references
    /// to temporaries the template declares.
    fn emit_move_view(&self, move_token: &TokenStream, prologue: &TokenStream) -> TokenStream {
        let body = self.emit_view_with(|view| {
            quote! {
                #topcoat_view::internal::MoveView::drive(#view).await
            }
        });
        quote! {
            #topcoat_view::internal::MoveView::new(async #move_token {
                #prologue
                #body
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
        self.emit_view_with(|view| view)
    }

    /// Emits this scope as a block that evaluates the scope's expressions
    /// in source order, builds its `JoinView`, and ends with `tail` applied
    /// to that view, inside the block.
    fn emit_view_with(&self, tail: impl FnOnce(TokenStream) -> TokenStream) -> TokenStream {
        let mut emitter = Emitter::new(false);
        for node in &self.nodes {
            node.emit(&mut emitter);
        }
        emitter.finish(tail)
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
        builder.finish().emit_root(false, false).to_string()
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
    fn a_root_view_is_a_scoped_move_view_driving_its_body() {
        let out = rendered(ViewBuilder::new());
        assert!(
            out.starts_with(":: topcoat_view :: internal :: ScopeView :: new ("),
            "{out}"
        );
        assert!(out.contains("MoveView :: new (async move"), "{out}");
        assert!(out.contains(":: drive (__view) . await"), "{out}");
    }

    #[test]
    fn a_self_contained_root_owns_its_buffer() {
        let out = ViewBuilder::new()
            .finish()
            .emit_root(true, true)
            .to_string();
        assert!(
            out.starts_with(":: topcoat_view :: internal :: ScopeView :: self_contained ("),
            "{out}"
        );
        assert!(out.contains("let __cx = & __cx ;"), "{out}");
        assert!(!out.contains("__buf"), "{out}");
        assert!(out.contains(":: drive (__view) . await"), "{out}");
    }

    #[test]
    fn a_root_view_is_driven_inside_the_template_block() {
        let mut builder = ViewBuilder::new();
        builder.local_binding(&syn::parse_quote!(x), &syn::parse_quote!(&value()));
        builder.expr(ExprKind::Node, quote! { x });
        let out = rendered(builder);
        // The binding borrows a temporary that lives until the end of its
        // block; the drive runs in that block, so the borrow is still valid.
        assert!(!out.contains("let __view = {"), "{out}");
        let block = out.find("{ let x = & value ()").expect(&out);
        let drive = out.find(":: drive (__view) . await }").expect(&out);
        assert!(block < drive, "{out}");
    }

    #[test]
    fn a_view_without_units_joins_over_unit() {
        let out = rendered(ViewBuilder::new());
        assert!(out.contains("JoinView :: new (__cx , () , move | __b , () |"));
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
    fn node_expression_is_classified_and_joined_as_a_unit() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p>");
        builder.expr(ExprKind::Node, quote! { value });
        builder.str_unescaped("</p>");
        let out = rendered(builder);
        assert!(
            out.contains(
                "let (__expr0_parts , __expr0) = :: topcoat_view :: internal :: NodeClassify :: classify (value)"
            ),
            "{out}"
        );
        assert!(out.contains("JoinUnit :: new (__expr0 , ())"), "{out}");
        assert!(out.contains("JoinView :: new"), "{out}");
        assert!(out.contains("move | __b , (__view0 , ()) |"), "{out}");
        assert!(out.contains("__b . markup (& \"<p>\")"), "{out}");
        assert!(
            out.contains("__b . node (__expr0_parts) ; __b . view (__view0)"),
            "{out}"
        );
        assert!(out.contains("__b . markup (& \"</p>\")"), "{out}");
    }

    #[test]
    fn if_else_without_components_splices_the_taken_branch_in_place() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.str_unescaped("yes");
            else_branch.str_unescaped("no");
        });
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = if cond"), "{out}");
        assert!(
            out.contains("ViewHandle :: unescaped_unchecked (\"yes\")"),
            "{out}"
        );
        assert!(
            out.contains("ViewHandle :: unescaped_unchecked (\"no\")"),
            "{out}"
        );
        assert!(out.contains("__b . view (__expr0)"), "{out}");
        assert!(!out.contains("EitherView"), "{out}");
        assert!(!out.contains("JoinUnit :: new (__expr0"), "{out}");
    }

    #[test]
    fn if_else_with_a_component_branch_wraps_the_branch_views_in_either() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            add_component(then_branch, "yes");
            else_branch.str_unescaped("no");
        });
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = if cond"), "{out}");
        assert!(out.contains("EitherView :: left"), "{out}");
        assert!(out.contains("EitherView :: right"), "{out}");
        assert!(out.contains("\"no\""), "{out}");
        assert!(out.contains("JoinUnit :: new (__expr0 , ())"), "{out}");
    }

    #[test]
    fn if_without_else_still_splices_an_else_view() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.str_unescaped("yes");
        });
        let out = rendered(builder);
        assert!(out.contains("if cond"), "{out}");
        assert!(
            out.contains("else { :: topcoat_view :: ViewHandle :: empty () }"),
            "{out}"
        );
    }

    #[test]
    fn for_loop_without_node_positions_builds_each_iteration_in_place() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.str_unescaped("<b");
            body.expr(ExprKind::AttributeUnescaped, quote! { ("id", x) });
            body.str_unescaped("></b>");
        });
        let out = rendered(builder);
        assert!(out.contains("for x in xs"), "{out}");
        assert!(out.contains("__views . push ("), "{out}");
        assert!(out.contains("Builder :: block (__cx , | __b |"), "{out}");
        assert!(out.contains("__b . attribute_unescaped (__expr0)"), "{out}");
        assert!(
            out.contains("for __view in __expr0 { __b . view (__view) ; }"),
            "{out}"
        );
        assert!(!out.contains("LoopView"), "{out}");
        assert!(!out.contains("Capture"), "{out}");
    }

    #[test]
    fn a_node_position_in_a_for_body_keeps_the_loop_joined() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.expr(ExprKind::Node, quote! { x });
        });
        let out = rendered(builder);
        assert!(out.contains("LoopView :: new (__iterations)"), "{out}");
        assert!(out.contains("NodeClassify :: classify (x)"), "{out}");
        assert!(out.contains("Capture ((x ,))"), "{out}");
    }

    #[test]
    fn control_flow_inside_a_plain_for_body_builds_in_place() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.str_unescaped("<b");
            body.if_else(&syn::parse_quote!(x.ok), |then_branch, _| {
                then_branch.expr(ExprKind::AttributeUnescaped, quote! { ("id", x.name) });
            });
            body.str_unescaped("></b>");
        });
        let out = rendered(builder);
        assert!(out.contains("let __expr0 = if x . ok"), "{out}");
        assert!(out.contains("__b . attribute_unescaped (__expr0)"), "{out}");
        assert!(out.contains("__b . view (__expr0)"), "{out}");
        assert!(!out.contains("EitherView"), "{out}");
        assert!(!out.contains("LoopView"), "{out}");
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
        assert!(out.contains("let __expr0 = match v"), "{out}");
        assert!(out.contains("A =>"), "{out}");
        assert!(out.contains("B if flag =>"), "{out}");
        assert!(out.contains("__b . view (__expr0)"), "{out}");
        assert!(!out.contains("EitherView"), "{out}");
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
        assert!(out.contains("let __expr0 = :: topcoat_view :: internal :: ThenView :: new"));
        assert!(out.contains("Component :: render"));
        assert!(out.contains("JoinUnit :: new (__expr0 , ())"));
    }

    #[test]
    fn children_pass_as_a_child_value() {
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
        assert!(out.contains("JoinUnit :: new (__expr0 , :: topcoat_view :: internal :: JoinUnit :: new (__expr1 , ()))"));
        assert!(out.contains("move | __b , (__view0 , (__view1 , ())) |"));
        assert!(out.contains("__b . view (__view0)"));
        assert!(out.contains("__b . view (__view1)"));
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
    fn for_loop_with_component_body_pins_each_iteration() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component(body, "item");
        });
        let out = rendered(builder);
        assert!(
            out.contains("__iterations . push (:: std :: boxed :: Box :: pin ("),
            "{out}"
        );
        assert!(out.contains("LoopView :: new (__iterations)"), "{out}");
        assert!(out.contains("ThenView :: new"), "{out}");
        assert!(out.contains("JoinUnit :: new (__expr0 , ())"), "{out}");
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
        let either = quote! { #topcoat_view::internal::EitherView }.to_string();
        assert!(out.contains(&format!("{either} :: left")));
        assert!(out.contains(&format!("{either} :: right ({either} :: left")));
        assert!(out.contains(&format!("{either} :: right ({either} :: right")));
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
