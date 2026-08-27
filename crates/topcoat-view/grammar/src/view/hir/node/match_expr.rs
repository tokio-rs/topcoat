use quote::{ToTokens, quote};
use syn::{Expr, Pat};

use crate::view::hir::{
    Scope,
    bindings::awaits,
    emit::{Emit, Emitter},
};

/// A `match` whose arm bodies are lowered into nested scopes.
pub(crate) struct MatchExpr {
    pub expr: Box<Expr>,
    pub arms: Vec<MatchArm>,
}

impl Emit for MatchExpr {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        let expr = self.expr.to_token_stream();
        let expr = if awaits(&expr) {
            Emitter::awaited(&expr)
        } else {
            expr
        };
        // The taken arm renders right inside the arm, where its pattern's
        // bindings are alive.
        emitter.control_flow(|emitter| {
            let arms: Vec<_> = self
                .arms
                .iter()
                .map(|arm| {
                    let pat = &arm.pat;
                    let guard = arm.guard.as_ref().map(|guard| quote! { if #guard });
                    let body = emitter.nested(&arm.body);
                    quote! { #pat #guard => { #body } }
                })
                .collect();
            quote! {
                match #expr {
                    #(#arms,)*
                }
            }
        });
    }
}

/// A single `pat (if guard)? => body` arm of a [`MatchExpr`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}
