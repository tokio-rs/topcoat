use quote::quote;
use syn::{Expr, Pat};

use crate::view::hir::emit::{Emit, Emitter};

/// A `let pat = expr;` binding, in scope for the nodes that follow it.
pub(crate) struct Local {
    pub pat: Pat,
    pub expr: Box<Expr>,
}

impl Emit for Local {
    fn emit(&self, emitter: &mut Emitter) {
        let pat = &self.pat;
        let expr = &self.expr;
        emitter.hoist(quote! { let #pat = #expr; });
    }
}
