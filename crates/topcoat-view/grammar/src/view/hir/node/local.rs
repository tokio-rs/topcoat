use proc_macro2::TokenStream;

use crate::view::hir::emit::{Emit, Emitter};

/// A `let pat = expr;` binding, in scope for the nodes that follow it.
pub(crate) struct Local {
    pub tokens: TokenStream,
}

impl Emit for Local {
    fn emit(&self, emitter: &mut Emitter) {
        emitter.hoist(self.tokens.clone());
    }
}
