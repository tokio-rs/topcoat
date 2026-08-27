use quote::{ToTokens, quote};
use syn::{Expr, Pat};

use crate::view::hir::{
    bindings::awaits,
    emit::{Emit, Emitter},
};

/// A `let pat = expr;` binding, in scope for the nodes that follow it.
pub(crate) struct Local {
    pub pat: Pat,
    pub expr: Box<Expr>,
}

impl Emit for Local {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        let pat = &self.pat;
        let expr = &self.expr;
        if awaits(&expr.to_token_stream()) {
            // The block is suspended for the duration of the statement,
            // which keeps the temporaries of the initializer where a `let`
            // puts them.
            emitter.push(quote! {
                __b.suspend();
                let #pat = #expr;
                __b.resume();
            });
        } else {
            emitter.push(quote! { let #pat = #expr; });
        }
    }
}
