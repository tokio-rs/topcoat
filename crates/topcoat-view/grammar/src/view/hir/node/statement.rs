use proc_macro2::TokenStream;

use crate::view::hir::emit::{Emit, Emitter};

/// A verbatim Rust statement.
pub(crate) struct Statement {
    pub tokens: TokenStream,
}

impl Emit for Statement {
    fn emit(&self, emitter: &mut Emitter) {
        emitter.hoist(self.tokens.clone());
    }
}
