use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Pat};
use topcoat_core_grammar::paths::topcoat_view;

use crate::view::hir::{
    Bindings, Scope,
    emit::{Emit, Emitter},
};

/// A `match` whose arm bodies are lowered into nested scopes.
pub(crate) struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

impl Emit for MatchExpr {
    fn emit(&self, emitter: &mut Emitter) {
        let ident = emitter.fresh_ident();
        let expr = &self.expr;

        if !self.arms.iter().any(|arm| arm.body.is_async()) {
            // The taken arm builds its block right inside the arm, where
            // its pattern's bindings are alive.
            let arms = self.arms.iter().map(|arm| {
                let pat = &arm.pat;
                let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                let body = arm.body.emit_block();
                quote! { #pat #guard => #body }
            });

            emitter.hoist(quote! {
                let #ident = match #expr {
                    #(#arms,)*
                };
            });
            emitter.burst(quote! { __b.view(#ident); });
            return;
        }

        // The arm bodies build views of different types; nested
        // `EitherView`s unify them, and only the taken arm is driven as this
        // position's unit. The arm's view takes its pattern's bindings with
        // it.
        let arm_count = self.arms.len();
        let arms = self.arms.iter().enumerate().map(|(index, arm)| {
            let pat = &arm.pat;
            let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
            let body = arm.body.emit_captured(&Bindings::of_pattern(pat));
            let body = nest_either(body, index, arm_count);
            quote! { #pat #guard => #body }
        });

        emitter.hoist(quote! {
            let #ident = match #expr {
                #(#arms,)*
            };
        });
        emitter.unit(Span::call_site(), &ident);
    }
}

/// Wraps one arm's view so all arms unify to the same nested `EitherView`
/// type: arm `i` of `n` becomes `i` `right`s around a `left`, and the last
/// arm `n - 1` `right`s around the bare view.
fn nest_either(view: TokenStream, index: usize, arm_count: usize) -> TokenStream {
    let last = index + 1 == arm_count;
    let mut tokens = if last {
        view
    } else {
        quote! { #topcoat_view::internal::EitherView::left(#view) }
    };
    for _ in 0..if last { arm_count - 1 } else { index } {
        tokens = quote! { #topcoat_view::internal::EitherView::right(#tokens) };
    }
    tokens
}

/// A single `pat (if guard)? => body` arm of a [`MatchExpr`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}
