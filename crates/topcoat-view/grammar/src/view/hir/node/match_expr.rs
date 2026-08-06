use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Expr, Pat};

use crate::view::hir::{
    Scope,
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

        let renders_components = self.arms.iter().any(|arm| arm.body.is_async());
        if emitter.inline_await() || !renders_components {
            let arms = self.arms.iter().map(|arm| {
                let pat = &arm.pat;
                let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                let body = arm.body.emit_expr();
                quote! { #pat #guard => #body }
            });

            emitter.hoist(quote! {
                let #ident = match #expr {
                    #(#arms,)*
                };
            });
        } else {
            // In a joined position the arm bodies yield futures instead of
            // views, so the taken arm joins with the scope's other
            // components. Nested `Either`s unify the arms' future types.
            let arm_count = self.arms.len();
            let arms = self.arms.iter().enumerate().map(|(index, arm)| {
                let pat = &arm.pat;
                let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                let body = nest_either(arm.body.emit_future(), index, arm_count);
                quote! { #pat #guard => #body }
            });

            emitter.hoist_result_future(
                Span::call_site(),
                &ident,
                &quote! {
                    match #expr {
                        #(#arms,)*
                    }
                },
            );
        }
        emitter.emit(quote! { __view(__cx, __parts, #ident); });
    }
}

/// Wraps one arm's future so all arms unify to the same nested `Either` type:
/// arm `i` of `n` becomes `i` `Right`s around a `Left`, and the last arm `n -
/// 1` `Right`s around the bare future.
fn nest_either(future: TokenStream, index: usize, arm_count: usize) -> TokenStream {
    let last = index + 1 == arm_count;
    let mut tokens = if last {
        future
    } else {
        quote! { __Either::Left(#future) }
    };
    for _ in 0..if last { arm_count - 1 } else { index } {
        tokens = quote! { __Either::Right(#tokens) };
    }
    tokens
}

/// A single `pat (if guard)? => body` arm of a [`MatchExpr`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}
