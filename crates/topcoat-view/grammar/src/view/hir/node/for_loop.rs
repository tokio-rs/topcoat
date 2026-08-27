use quote::{ToTokens, quote};
use syn::{Expr, Pat};

use crate::view::hir::{
    Scope,
    bindings::awaits,
    emit::{Emit, Emitter},
};

/// A `for` loop whose body is lowered into a nested scope.
pub(crate) struct ForLoop {
    pub pat: Pat,
    pub expr: Box<Expr>,
    pub body: Scope,
}

impl Emit for ForLoop {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        let Self { pat, expr, body } = self;
        let expr = expr.to_token_stream();
        let expr = if awaits(&expr) {
            Emitter::awaited(&expr)
        } else {
            expr
        };
        // Each iteration renders right inside the loop, where the
        // pattern's bindings are alive; what it leaves to drive collects
        // into the template's `Vec`s.
        emitter.control_flow(|emitter| {
            let body = emitter.nested(body);
            quote! {
                for #pat in #expr {
                    #body
                }
            }
        });
    }
}
