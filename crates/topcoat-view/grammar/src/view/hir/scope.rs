use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;
use topcoat_core_grammar::paths::topcoat_view;

use super::{
    Bindings, Node,
    emit::{Emit, Emitter, Placement, Sites},
};

/// The lowered form of a `view!` invocation: the HIR between the view AST and
/// the emitted `TokenStream`. Built by [`ViewBuilder`](super::ViewBuilder).
pub(crate) struct Scope {
    nodes: Vec<Node>,
    /// How often the scope renders per pass over its template.
    placement: Placement,
}

impl Scope {
    pub(super) fn new(nodes: Vec<Node>, placement: Placement) -> Self {
        Self { nodes, placement }
    }

    /// Whether any expression of this scope, or of a scope nested in it,
    /// mentions `ident`.
    pub(super) fn mentions(&self, ident: &Ident) -> bool {
        self.nodes.iter().any(|node| node.mentions(ident))
    }

    /// Emits this scope's nodes as straight-line code pushing through the
    /// `__b` builder, storing what its node positions leave to drive into
    /// `sites`.
    pub(super) fn emit_nodes(&self, sites: &mut Sites) -> TokenStream {
        let mut emitter = Emitter::new(sites, self.placement);
        for node in &self.nodes {
            node.emit(&mut emitter);
        }
        emitter.finish()
    }

    /// Emits the body of a template: the code that builds the template's
    /// block against the ambient `__cx` context and drives what its node
    /// positions left to drive.
    ///
    /// The builder is opened at the start and closed at the end, so the
    /// nodes' code runs with the block open and may declare bindings the
    /// template's views borrow: those bindings live as long as the body,
    /// and the views are driven inside it.
    fn emit_body(&self) -> TokenStream {
        let mut sites = Sites::new();
        let nodes = self.emit_nodes(&mut sites);
        let pending = sites.tuple();
        quote! {
            #[allow(unused_mut)]
            let mut __b = #topcoat_view::internal::Builder::open(__cx);
            #nodes
            let __content = __b.close();
            let __view = #topcoat_view::internal::TemplateView::new(__content, #pending);
            #topcoat_view::internal::drive(__view).await
        }
    }

    /// Emits a top-level `view!` invocation: a `ScopeView` around a
    /// `MoveView` whose `async move` body builds the template and drives
    /// it in place.
    ///
    /// The block captures every value the template uses, so the view owns
    /// its data and the expressions inside borrow from the block. The
    /// `ScopeView` makes the view own the buffer of the build when it is
    /// the outermost view; nested inside another build, it appends to that
    /// build's buffer.
    ///
    /// With `owns_cx`, the block expects an owned `__cx` context in scope
    /// and captures it, rebinding `__cx` to a borrow of it inside; the view
    /// then does not borrow the caller's context. With `self_contained`,
    /// the view always owns a buffer of its own, so its content renders
    /// anywhere even when it is built inside another build.
    pub fn emit_root(&self, owns_cx: bool, self_contained: bool) -> TokenStream {
        let prologue = owns_cx.then(|| quote! { let __cx = &__cx; });
        let body = self.emit_body();
        let view = quote! {
            #topcoat_view::internal::MoveView::new(async move {
                #prologue
                #body
            })
        };
        if self_contained {
            quote! { #topcoat_view::internal::ScopeView::self_contained(#view) }
        } else {
            quote! { #topcoat_view::internal::ScopeView::new(#view) }
        }
    }

    /// Emits this scope as the child content of a component invocation
    /// that sits under the patterns binding `captures`: a `MoveView` whose
    /// body builds the template and drives it in place.
    ///
    /// The body is not `move`, so it borrows its environment and a value
    /// shared by all iterations of an enclosing loop is not moved into the
    /// first. The captured bindings are the exception: they die with the
    /// branch or iteration that produced them while the child lives on in
    /// the component's props, so the child takes them along in a `Capture`
    /// packed where they are still alive and taken back apart inside the
    /// body.
    pub(crate) fn emit_child(&self, captures: &Bindings) -> TokenStream {
        let body = self.emit_body();
        if captures.is_empty() {
            return quote! { #topcoat_view::internal::MoveView::new(async { #body }) };
        }
        let idents = captures.idents();
        let rebinds = captures.rebinds();
        quote! {{
            let __captured = #topcoat_view::internal::Capture((#(#idents,)*));
            #topcoat_view::internal::MoveView::new(async {
                let (#(#rebinds,)*) = __captured.take();
                #body
            })
        }}
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::Span;
    use quote::quote;
    use syn::Expr;

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
    fn a_root_view_is_a_scoped_move_view_driving_its_template() {
        let out = rendered(ViewBuilder::new());
        assert!(
            out.starts_with(":: topcoat_view :: internal :: ScopeView :: new ("),
            "{out}"
        );
        assert!(out.contains("MoveView :: new (async move"), "{out}");
        assert!(out.contains("Builder :: open (__cx)"), "{out}");
        assert!(out.contains("let __content = __b . close ()"), "{out}");
        assert!(
            out.contains("let __view = :: topcoat_view :: internal :: TemplateView :: new (__content , ()) ; :: topcoat_view :: internal :: drive (__view) . await"),
            "{out}"
        );
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
        assert!(out.contains("Builder :: open (__cx)"), "{out}");
    }

    #[test]
    fn the_template_runs_inside_the_block_it_is_driven_in() {
        let mut builder = ViewBuilder::new();
        builder.local_binding(&syn::parse_quote!(x), &syn::parse_quote!(&value()));
        builder.expr(ExprKind::Node, quote! { x });
        let out = rendered(builder);
        // The binding borrows a temporary that lives until the end of its
        // block; the drive runs in that block, so the borrow is still valid.
        let open = out.find("Builder :: open (__cx)").expect(&out);
        let local = out.find("let x = & value ()").expect(&out);
        let drive = out.find(":: drive (").expect(&out);
        assert!(open < local && local < drive, "{out}");
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
    fn a_node_position_renders_in_place_and_keeps_its_pending() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<p>");
        builder.expr(ExprKind::Node, quote! { value });
        builder.str_unescaped("</p>");
        let out = rendered(builder);
        assert!(out.contains("__b . markup (& \"<p>\")"), "{out}");
        assert!(
            out.contains(
                "let __s0 = :: topcoat_view :: internal :: NodePosition :: render (value , & mut __b) ;"
            ),
            "{out}"
        );
        assert!(out.contains("__b . markup (& \"</p>\")"), "{out}");
        assert!(
            out.contains("TemplateView :: new (__content , (__s0 ,))"),
            "{out}"
        );
    }

    #[test]
    fn positions_are_numbered_in_source_order_across_nested_scopes() {
        let mut builder = ViewBuilder::new();
        builder.expr(ExprKind::Node, quote! { a });
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.expr(ExprKind::Node, quote! { b });
        });
        builder.expr(ExprKind::Node, quote! { c });
        let out = rendered(builder);
        assert!(out.contains("let __s0 = "), "{out}");
        assert!(
            out.contains("__s1 = :: core :: option :: Option :: Some ("),
            "{out}"
        );
        assert!(out.contains("let __s2 = "), "{out}");
        assert!(
            out.contains("TemplateView :: new (__content , (__s0 , __s1 , __s2 ,))"),
            "{out}"
        );
    }

    #[test]
    fn if_else_renders_the_taken_branch_in_place() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            then_branch.str_unescaped("yes");
            else_branch.str_unescaped("no");
        });
        let out = rendered(builder);
        assert!(
            out.contains(
                "if cond { __b . markup (& \"yes\") ; } else { __b . markup (& \"no\") ; }"
            ),
            "{out}"
        );
    }

    #[test]
    fn if_without_else_emits_no_else_branch() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            then_branch.str_unescaped("yes");
        });
        let out = rendered(builder);
        assert!(
            out.contains("if cond { __b . markup (& \"yes\") ; }"),
            "{out}"
        );
        assert!(!out.contains("else"), "{out}");
    }

    #[test]
    fn a_position_in_a_branch_collects_into_an_option() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, else_branch| {
            add_component(then_branch, "yes");
            else_branch.expr(ExprKind::Node, quote! { no });
        });
        let out = rendered(builder);
        assert!(
            out.contains("let mut __s0 = :: core :: option :: Option :: None ;"),
            "{out}"
        );
        assert!(
            out.contains("let mut __s1 = :: core :: option :: Option :: None ;"),
            "{out}"
        );
        assert!(
            out.contains("__s0 = :: core :: option :: Option :: Some (:: topcoat_view :: internal :: NodePosition :: render (:: topcoat_view :: internal :: ThenView :: new ("),
            "{out}"
        );
        assert!(
            out.contains("__s1 = :: core :: option :: Option :: Some (:: topcoat_view :: internal :: NodePosition :: render (no , & mut __b))"),
            "{out}"
        );
        assert!(out.contains("(__s0 , __s1 ,)"), "{out}");
    }

    #[test]
    fn for_loop_renders_each_iteration_in_place() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.str_unescaped("<b");
            body.expr(ExprKind::AttributeUnescaped, quote! { ("id", x) });
            body.str_unescaped("></b>");
        });
        let out = rendered(builder);
        assert!(
            out.contains("for x in xs { __b . markup (& \"<b\") ; __b . attribute_unescaped ((\"id\" , x)) ; __b . markup (& \"></b>\") ; }"),
            "{out}"
        );
        assert!(
            out.contains("TemplateView :: new (__content , ())"),
            "{out}"
        );
    }

    #[test]
    fn collectors_are_declared_right_before_their_outermost_control_flow() {
        let mut builder = ViewBuilder::new();
        builder.local_binding(&syn::parse_quote!(items), &syn::parse_quote!(load()));
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(&items), |body| {
            body.if_else(&syn::parse_quote!(x.ok), |then_branch, _| {
                add_component(then_branch, "card");
            });
            body.for_loop(&syn::parse_quote!(y), &syn::parse_quote!(ys), |body| {
                body.expr(ExprKind::Node, quote! { y });
            });
        });
        let out = rendered(builder);
        // Declared after the binding the collected views may borrow, and
        // only once, ahead of the outermost loop.
        assert!(
            out.contains("let items = load () ; let mut __s0 = :: std :: vec :: Vec :: new () ; let mut __s1 = :: std :: vec :: Vec :: new () ; for x in & items {"),
            "{out}"
        );
        assert_eq!(out.matches("Vec :: new ()").count(), 2, "{out}");
    }

    #[test]
    fn an_awaiting_local_suspends_the_block() {
        let mut builder = ViewBuilder::new();
        builder.local_binding(&syn::parse_quote!(x), &syn::parse_quote!(load().await?));
        builder.statement(quote! { count += 1; });
        builder.statement(quote! { wait().await; });
        let out = rendered(builder);
        assert!(
            out.contains("__b . suspend () ; let x = load () . await ? ; __b . resume () ; count += 1 ; __b . suspend () ; wait () . await ; __b . resume () ;"),
            "{out}"
        );
    }

    #[test]
    fn an_awaiting_expression_evaluates_with_the_block_suspended() {
        let mut builder = ViewBuilder::new();
        builder.expr(ExprKind::Node, quote! { load().await });
        builder.expr(ExprKind::AttributeValue, quote! { attr().await });
        let out = rendered(builder);
        assert!(
            out.contains("NodePosition :: render (__b . suspended () . resumed (load () . await) , & mut __b)"),
            "{out}"
        );
        assert!(
            out.contains("let __awaited = __b . suspended () . resumed (attr () . await) ; __b . attribute_value (__awaited) ;"),
            "{out}"
        );
    }

    #[test]
    fn awaiting_props_build_the_render_future_with_the_block_suspended() {
        let mut builder = ViewBuilder::new();
        let path = syn::parse_str("card").unwrap();
        let arg = NamedArg {
            ident: syn::parse_quote!(item),
            colon: syn::token::Colon::default(),
            value: NamedArgValue::Expr(syn::parse_quote!(load().await?)),
        };
        builder.component(
            &path,
            vec![arg],
            None,
            &syn::parse_quote!(),
            Span::call_site(),
        );
        let out = rendered(builder);
        assert!(
            out.contains("NodePosition :: render (__b . suspended () . resumed (:: topcoat_view :: internal :: ThenView :: new ("),
            "{out}"
        );
        assert!(out.contains("props ,) }))) , & mut __b)"), "{out}");
    }

    #[test]
    fn an_await_in_children_does_not_suspend_the_parent() {
        let mut builder = ViewBuilder::new();
        add_component_with_children(
            &mut builder,
            "wrapper",
            &syn::parse_quote!({
                let x = load().await;
            }(x)),
        );
        let out = rendered(builder);
        assert_eq!(out.matches("__b . suspend ()").count(), 1, "{out}");
        assert!(
            out.contains("__b . suspend () ; let x = load () . await ; __b . resume () ;"),
            "{out}"
        );
    }

    #[test]
    fn awaiting_control_flow_heads_evaluate_with_the_block_suspended() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(
            &syn::parse_quote!(x),
            &syn::parse_quote!(load().await),
            |_| {},
        );
        builder.match_expr(&syn::parse_quote!(pick().await), |_| {});
        builder.if_else(
            &syn::parse_quote!(flag && let Some(y) = find().await && y.ok),
            |_, _| {},
        );
        let out = rendered(builder);
        assert!(
            out.contains("for x in __b . suspended () . resumed (load () . await) {"),
            "{out}"
        );
        assert!(
            out.contains("match __b . suspended () . resumed (pick () . await) {"),
            "{out}"
        );
        assert!(
            out.contains("if flag && let Some (y) = __b . suspended () . resumed (find () . await) && y . ok {"),
            "{out}"
        );
    }

    #[test]
    fn a_position_in_a_for_body_collects_into_a_vec() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.expr(ExprKind::Node, quote! { x });
        });
        let out = rendered(builder);
        assert!(
            out.contains("let mut __s0 = :: std :: vec :: Vec :: new () ;"),
            "{out}"
        );
        assert!(
            out.contains("for x in xs { __s0 . push (:: topcoat_view :: internal :: NodePosition :: render (x , & mut __b)) ; }"),
            "{out}"
        );
        assert!(out.contains("(__s0 ,)"), "{out}");
    }

    #[test]
    fn branches_inside_a_for_body_still_collect_into_a_vec() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.if_else(&syn::parse_quote!(x.ok), |then_branch, _| {
                add_component(then_branch, "card");
            });
        });
        let out = rendered(builder);
        assert!(
            out.contains("let mut __s0 = :: std :: vec :: Vec :: new () ;"),
            "{out}"
        );
        assert!(out.contains("if x . ok { __s0 . push ("), "{out}");
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
        assert!(
            out.contains("match v { A => { __b . markup (& \"a\") ; } , B if flag => { __b . markup (& \"b\") ; } , }"),
            "{out}"
        );
    }

    #[test]
    fn match_arms_collect_into_their_own_options() {
        let mut builder = ViewBuilder::new();
        builder.match_expr(&syn::parse_quote!(v), |arms| {
            arms.arm(&syn::parse_quote!(A), None, |body| {
                add_component(body, "a");
            });
            arms.arm(&syn::parse_quote!(B), None, |body| {
                add_component(body, "b");
            });
        });
        let out = rendered(builder);
        assert!(
            out.contains("A => { __s0 = :: core :: option :: Option :: Some ("),
            "{out}"
        );
        assert!(
            out.contains("B => { __s1 = :: core :: option :: Option :: Some ("),
            "{out}"
        );
        assert!(out.contains("(__s0 , __s1 ,)"), "{out}");
    }

    #[test]
    fn local_binding_emits_let_statement() {
        let mut builder = ViewBuilder::new();
        builder.local_binding(&syn::parse_quote!(x), &syn::parse_quote!(value));
        builder.str_unescaped("ok");
        let out = rendered(builder);
        assert!(out.contains("let x = value ;"));
    }

    #[test]
    fn a_statement_is_emitted_verbatim_in_source_order() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("a");
        builder.statement(quote! { total += 1; });
        builder.str_unescaped("b");
        let out = rendered(builder);
        assert!(
            out.contains("__b . markup (& \"a\") ; total += 1 ; __b . markup (& \"b\") ;"),
            "{out}"
        );
    }

    #[test]
    fn a_component_renders_as_a_then_view_at_its_position() {
        let mut builder = ViewBuilder::new();
        add_component(&mut builder, "solo");
        builder.str_unescaped("<hr>");
        let out = rendered(builder);
        assert!(
            out.contains("let __s0 = :: topcoat_view :: internal :: NodePosition :: render (:: topcoat_view :: internal :: ThenView :: new ("),
            "{out}"
        );
        assert!(out.contains("Component :: render"), "{out}");
        assert!(
            out.contains("& mut __b) ; __b . markup (& \"<hr>\") ;"),
            "{out}"
        );
    }

    #[test]
    fn children_pass_as_a_child_value_driving_their_own_template() {
        let mut builder = ViewBuilder::new();
        add_component_with_children(&mut builder, "wrapper", &syn::parse_quote!(inner()));
        let out = rendered(builder);
        assert!(
            out.contains(". child (:: topcoat_view :: Child :: new ("),
            "{out}"
        );
        // The child borrows its environment and is passed unpolled: the
        // wrapper decides where, and whether, it is driven.
        assert!(
            out.contains("Child :: new (:: topcoat_view :: internal :: MoveView :: new (async {"),
            "{out}"
        );
        assert_eq!(out.matches("async move").count(), 1, "{out}");
        assert_eq!(out.matches("Builder :: open (__cx)").count(), 2, "{out}");
        assert!(!out.contains("Capture"), "{out}");
    }

    #[test]
    fn children_in_a_for_body_capture_the_pattern_bindings_they_mention() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!((i, x)), &syn::parse_quote!(xs), |body| {
            add_component_with_children(
                body,
                "wrapper",
                &syn::parse_quote!(inner(index: i, item: &x)),
            );
        });
        let out = rendered(builder);
        assert!(out.contains("Capture ((i , x ,))"), "{out}");
        assert!(
            out.contains("let (i , x ,) = __captured . take ()"),
            "{out}"
        );
    }

    #[test]
    fn children_leave_the_bindings_they_do_not_mention_alone() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!((i, x)), &syn::parse_quote!(xs), |body| {
            add_component_with_children(
                body,
                "wrapper",
                &syn::parse_quote!(inner()(format!("{x}"))),
            );
        });
        let out = rendered(builder);
        assert!(out.contains("Capture ((x ,))"), "{out}");
        assert!(!out.contains("Capture ((i , x ,))"), "{out}");
    }

    #[test]
    fn children_under_nested_patterns_capture_every_binding() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            body.if_else(
                &syn::parse_quote!(let Some(status) = x.status),
                |then_branch, _| {
                    then_branch.match_expr(&syn::parse_quote!(status), |arms| {
                        arms.arm(&syn::parse_quote!(Some(inner)), None, |body| {
                            add_component_with_children(
                                body,
                                "wrapper",
                                &syn::parse_quote!(child(a: x, b: status, c: inner)),
                            );
                        });
                    });
                },
            );
        });
        let out = rendered(builder);
        assert!(out.contains("Capture ((x , status , inner ,))"), "{out}");
    }

    #[test]
    fn children_do_not_capture_bindings_of_their_own_template() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component_with_children(
                body,
                "wrapper",
                &syn::parse_quote!(for y in ys { inner(child(a: x, b: y)) }),
            );
        });
        let out = rendered(builder);
        // The outer child owns `x`; its own nested child captures only the
        // binding of the loop inside it.
        assert!(out.contains("Capture ((x ,))"), "{out}");
        assert!(out.contains("Capture ((y ,))"), "{out}");
        assert!(!out.contains("Capture ((x , y ,))"), "{out}");
    }

    #[test]
    fn a_branch_without_bindings_captures_nothing_for_its_children() {
        let mut builder = ViewBuilder::new();
        builder.if_else(&syn::parse_quote!(cond), |then_branch, _| {
            add_component_with_children(then_branch, "wrapper", &syn::parse_quote!(inner()));
        });
        let out = rendered(builder);
        assert!(!out.contains("Capture"), "{out}");
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
    fn a_for_loop_body_borrows_from_the_root_template() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&syn::parse_quote!(x), &syn::parse_quote!(xs), |body| {
            add_component(body, "item");
        });
        let out = rendered(builder);
        // Only the root owns its captures; the iterations run inside it,
        // where the pattern's bindings are alive.
        assert_eq!(out.matches("async move").count(), 1);
        assert_eq!(out.matches("async").count(), 1);
        assert!(!out.contains("Capture"));
        assert!(!out.contains("Box :: pin"));
    }

    #[test]
    fn many_positions_nest_their_pending_tuple() {
        let mut builder = ViewBuilder::new();
        for _ in 0..20 {
            builder.expr(ExprKind::Node, quote! { v });
        }
        let out = rendered(builder);
        assert!(
            out.contains("(__s0 , __s1 , __s2 , __s3 , __s4 , __s5 , __s6 , __s7 , __s8 , __s9 , __s10 , __s11 , __s12 , __s13 , __s14 , (__s15 , __s16 , __s17 , __s18 , __s19 ,) ,)"),
            "{out}"
        );
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
            let expected = format!("__b . {method} (v)");
            assert!(
                rendered(builder).contains(&expected),
                "expected builder call `{expected}`",
            );
        }
    }
}
