use proc_macro2::TokenStream;
use quote::quote;

use crate::view::hir::{
    bindings::awaits,
    emit::{Emit, Emitter},
};

/// A verbatim Rust statement.
pub(crate) struct Statement {
    pub tokens: TokenStream,
}

impl Emit for Statement {
    fn emit(&self, emitter: &mut Emitter<'_>) {
        let tokens = &self.tokens;
        if awaits(tokens) {
            emitter.push(quote! {
                __b.suspend();
                #tokens
                __b.resume();
            });
        } else {
            emitter.push(tokens.clone());
        }
    }
}
