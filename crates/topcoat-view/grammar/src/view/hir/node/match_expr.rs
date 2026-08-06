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
        emitter.emit(quote! { __view(__cx, __parts, #ident); });
    }
}

/// A single `pat (if guard)? => body` arm of a [`MatchExpr`].
pub(crate) struct MatchArm {
    pub pat: Pat,
    pub guard: Option<Expr>,
    pub body: Scope,
}
