//! Emission targeting the streaming pass runtime: the render loop form.
//!
//! The counterpart of [`emit`](super::emit) for the component future model.
//! Where the joined form builds a view once, this form emits the body of a
//! render loop that runs once per pass: statements in source order, every
//! user expression inside a closure that is not async so render code cannot
//! await, and child invocations advanced through the children table. The
//! hand written corpus in `topcoat-view/tests/pass.rs` is the shape this
//! emission reproduces.

// Wires in when the macros flip to the component future model; until then
// only the unit tests exercise it.
#![allow(dead_code)]

use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use topcoat_core_grammar::paths::{topcoat_error, topcoat_view};

use super::{Component, ExprKind, Node, Scope};

impl Scope {
    /// Emits this scope as a component's render loop: the returning
    /// expression of a component future. Renders once per pass and suspends
    /// at the pass boundary; the expression never returns normally, so it
    /// fits any tail position whose function returns a `Result`.
    pub fn emit_pass_root(&self) -> TokenStream {
        let statements = self.emit_pass_statements();
        quote! {{
            let __mount = #topcoat_view::pass::mount();
            let mut __children = #topcoat_view::pass::Children::new();
            loop {
                let mut __out = #topcoat_view::pass::RenderBuffer::new();
                #statements
                __children.sweep();
                __mount.finish_render(__out);
                #topcoat_view::pass::pass_boundary(&__mount, &mut __children).await?;
            }
        }}
    }

    /// Emits this scope's nodes as render statements over the enclosing
    /// `__out` and `__children` bindings, in source order.
    fn emit_pass_statements(&self) -> TokenStream {
        let mut body = TokenStream::new();
        for node in self.nodes() {
            emit_pass_node(node, &mut body);
        }
        body
    }
}

/// Wraps a user expression in a closure that is not async, so `.await` in
/// render position does not compile while `?` keeps propagating: first out
/// of the closure, then out of the component through the enclosing render
/// statement.
fn render_expr(tokens: &TokenStream) -> TokenStream {
    quote! {{
        let __render = || -> ::core::result::Result<_, #topcoat_error::Error> {
            ::core::result::Result::Ok(#tokens)
        };
        __render()?
    }}
}

fn emit_pass_node(node: &Node, body: &mut TokenStream) {
    match node {
        Node::StaticSegment(segment) => {
            let string = &segment.string;
            body.extend(quote! { __out.markup_static(&#string); });
        }
        Node::ExprNode(expr) => {
            let value = render_expr(&expr.tokens);
            let method = pass_method(expr.kind);
            body.extend(quote! { __out.#method(#value); });
        }
        Node::Local(local) => {
            // A binding emits plainly so temporary lifetime extension keeps
            // working (`let s = &Signal::new(..)`); the await guard runs at
            // expansion time instead.
            let pat = &local.pat;
            let expr = &local.expr;
            let tokens = expr.to_token_stream();
            if tokens_contain_await(&tokens) {
                body.extend(quote! {
                    ::core::compile_error!(
                        "`.await` is not allowed in render code: loads run in \
                         the body, once, or through `defer`"
                    );
                });
            } else {
                body.extend(quote! { let #pat = #tokens; });
            }
        }
        Node::Statement(statement) => {
            let tokens = &statement.tokens;
            body.extend(quote! { #tokens });
        }
        Node::ForLoop(for_loop) => {
            let pat = &for_loop.pat;
            let iter = render_expr(&for_loop.expr.to_token_stream());
            let loop_body = for_loop.body.emit_pass_statements();
            body.extend(quote! {
                for #pat in #iter {
                    #loop_body
                }
            });
        }
        Node::IfElse(if_else) => {
            // A `let` condition binds into the branch, so it cannot move into
            // the render closure; it emits unwrapped.
            let condition = if contains_let(&if_else.expr) {
                if_else.expr.to_token_stream()
            } else {
                render_expr(&if_else.expr.to_token_stream())
            };
            let then_branch = if_else.then_branch.emit_pass_statements();
            let else_branch = if_else.else_branch.emit_pass_statements();
            body.extend(quote! {
                if #condition {
                    #then_branch
                } else {
                    #else_branch
                }
            });
        }
        Node::MatchExpr(match_expr) => {
            let scrutinee = render_expr(&match_expr.expr.to_token_stream());
            let arms = match_expr.arms.iter().map(|arm| {
                let pat = &arm.pat;
                let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                let arm_body = arm.body.emit_pass_statements();
                quote! { #pat #guard => { #arm_body } }
            });
            body.extend(quote! {
                match #scrutinee {
                    #(#arms,)*
                }
            });
        }
        Node::Component(component) => emit_pass_component(component, body),
        Node::Place(tokens) => {
            body.extend(quote! { __out.place(#tokens)?; });
        }
    }
}

fn emit_pass_component(component: &Component, body: &mut TokenStream) {
    let span = component.span;
    let path = &component.path;

    // A component's identity comes from its invocation site, so a site that
    // repeats cannot tell its repetitions apart without a key.
    if component.key.is_none() && component.repeats {
        let label = path
            .to_token_stream()
            .to_string()
            .replace(char::is_whitespace, "");
        let message = format!(
            "`{label}` repeats without a `key` argument: pass `key:` to give \
             each repetition its own identity"
        );
        body.extend(quote_spanned! {span=> compile_error!(#message); });
        return;
    }

    let site = component.site();
    let content_site = content_site(component);
    let setters = component.named_args.iter().map(|arg| {
        let ident = &arg.ident;
        let value = render_expr(&arg.value.to_token_stream());
        quote! { .#ident(#value) }
    });
    let content = component.children.as_ref().map(Scope::emit_pass_content);

    // Props and content evaluate once, on the birth pass; later passes only
    // advance what was born, so expressions that move values run once.
    let (existing_content, birth_content, child_setter) = match &content {
        Some(content) => (
            quote_spanned! {span=>
                let _ = __children.content_existing(#content_site)?;
            },
            quote_spanned! {span=>
                let __content = __children
                    .content(#content_site, || ::core::result::Result::Ok(#content))?;
            },
            quote_spanned! {span=> .child(__content) },
        ),
        None => (TokenStream::new(), TokenStream::new(), TokenStream::new()),
    };

    let render = quote_spanned! {span=> {
        use #topcoat_view::Component;
        let props = #path::props_builder()#(#setters)*#child_setter.build();
        // The marker is built via `Default` so the same construction works
        // for both unit-struct and generic (`PhantomData`) markers.
        #[allow(clippy::default_constructed_unit_structs)]
        ::core::result::Result::Ok(Component::render(
            #path::default(),
            __cx.detach(),
            props,
        ))
    }};

    let advance = if let Some(arg) = &component.key {
        let ident = &arg.ident;
        let value = &arg.value;
        let (existing_content, birth_content) = match &content {
            Some(_) => (
                quote_spanned! {span=>
                    let _ = __children.content_existing_keyed(#content_site, &__key)?;
                },
                quote_spanned! {span=>
                    let __content = __children
                        .content_keyed(#content_site, &__key, || {
                            ::core::result::Result::Ok(#content)
                        })?;
                },
            ),
            None => (TokenStream::new(), TokenStream::new()),
        };
        quote_spanned! {span=> {
            use #topcoat_view::Component;
            let mut __key = ::core::option::Option::None;
            #path::props_builder()
                .#ident(#value, |__value| __key = ::core::option::Option::Some(__value));
            let __key = __key.unwrap();
            if __children.has_keyed(#site, &__key) {
                #existing_content
                __children.advance_existing_keyed(&mut __out, #site, __key)?;
            } else {
                #birth_content
                __children.advance_keyed(&mut __out, #site, __key, || #render)?;
            }
        }}
    } else {
        quote_spanned! {span=> {
            if __children.has(#site) {
                #existing_content
                __children.advance_existing(&mut __out, #site)?;
            } else {
                #birth_content
                __children.advance(&mut __out, #site, || #render)?;
            }
        }}
    };
    body.extend(advance);
}

/// The content block's own invocation site: the component's site with the
/// high ordinal bit set, so the content and the invocation never share an
/// identity.
fn content_site(component: &Component) -> TokenStream {
    let ordinal = component.ordinal | 0x8000_0000;
    quote_spanned! {component.span=>
        const {
            #topcoat_view::identity::SiteKey::new(
                ::core::file!(),
                ::core::line!(),
                ::core::column!(),
                #ordinal,
            )
        }
    }
}

impl Scope {
    /// Emits this scope as the future of an anonymous content component:
    /// the compiled form of a trailing child block, owned and advanced by
    /// its creator, placed by the receiver through its token.
    fn emit_pass_content(&self) -> TokenStream {
        let statements = self.emit_pass_statements();
        // Content captures by move, the way props do: what the block reads
        // from the creator travels in, cloned by the creator if it also
        // keeps it.
        quote! {
            async move {
                let __mount = #topcoat_view::pass::mount();
                let mut __children = #topcoat_view::pass::Children::new();
                loop {
                    let mut __out = #topcoat_view::pass::RenderBuffer::new();
                    #statements
                    __children.sweep();
                    __mount.finish_render(__out);
                    #topcoat_view::pass::pass_boundary(&__mount, &mut __children).await?;
                }
            }
        }
    }
}

/// Whether the tokens contain an `await`, at any nesting depth.
fn tokens_contain_await(tokens: &TokenStream) -> bool {
    tokens.clone().into_iter().any(|tt| match tt {
        proc_macro2::TokenTree::Ident(ident) => ident == "await",
        proc_macro2::TokenTree::Group(group) => tokens_contain_await(&group.stream()),
        _ => false,
    })
}

fn pass_method(kind: ExprKind) -> proc_macro2::Ident {
    // The pass render buffer mirrors the builder's position methods.
    kind.builder_method()
}

#[cfg(test)]
mod tests {
    use quote::quote;
    use syn::parse_quote;

    use crate::view::hir::ViewBuilder;

    fn rendered(builder: ViewBuilder) -> String {
        builder.finish().emit_pass_root().to_string()
    }

    #[test]
    fn root_is_a_render_loop() {
        let mut builder = ViewBuilder::new();
        builder.text("hello");
        let output = rendered(builder);
        assert!(output.contains("pass :: mount ()"), "{output}");
        assert!(output.contains("loop"), "{output}");
        assert!(output.contains("__children . sweep ()"), "{output}");
        assert!(output.contains("finish_render (__out)"), "{output}");
        assert!(
            output.contains("pass_boundary (& __mount , & mut __children) . await ?"),
            "{output}"
        );
    }

    #[test]
    fn static_markup_is_promoted() {
        let mut builder = ViewBuilder::new();
        builder.str_unescaped("<h1>");
        let output = rendered(builder);
        assert!(
            output.contains("__out . markup_static (& \"<h1>\")"),
            "{output}"
        );
    }

    #[test]
    fn expressions_render_inside_a_closure_that_is_not_async() {
        let mut builder = ViewBuilder::new();
        builder.expr(crate::view::hir::ExprKind::Node, quote! { value });
        let output = rendered(builder);
        assert!(output.contains("__out . node"), "{output}");
        assert!(
            output.contains("let __render = || -> :: core :: result :: Result"),
            "{output}"
        );
        assert!(output.contains("__render () ?"), "{output}");
    }

    #[test]
    fn control_flow_recurses_with_wrapped_scrutinees() {
        let mut builder = ViewBuilder::new();
        builder.for_loop(&parse_quote!(item), &parse_quote!(items), |body| {
            body.text("row");
        });
        let output = rendered(builder);
        assert!(output.contains("for item in"), "{output}");
        assert!(output.contains("__render () ?"), "{output}");
        assert!(output.contains("markup_static (& \"row\")"), "{output}");
    }

    #[test]
    fn components_advance_through_the_children_table() {
        let mut builder = ViewBuilder::new();
        builder.component(
            &parse_quote!(card),
            Vec::new(),
            None,
            &parse_quote!(),
            proc_macro2::Span::call_site(),
        );
        let output = rendered(builder);
        assert!(
            output.contains("__children . advance (& mut __out"),
            "{output}"
        );
        assert!(output.contains("Component :: render"), "{output}");
        assert!(output.contains("__cx . detach ()"), "{output}");
    }
}

/// Whether a condition contains a `let` binding, including through the
/// operands of a let chain.
fn contains_let(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Let(_) => true,
        syn::Expr::Binary(binary) => contains_let(&binary.left) || contains_let(&binary.right),
        syn::Expr::Paren(paren) => contains_let(&paren.expr),
        syn::Expr::Group(group) => contains_let(&group.expr),
        syn::Expr::Unary(unary) => contains_let(&unary.expr),
        _ => false,
    }
}
