use proc_macro2::TokenStream;
use quote::TokenStreamExt;

pub trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

pub struct Emitter {
    hoist: TokenStream,
    emit: TokenStream,
}

impl Emitter {
    pub(super) fn hoist(&mut self, tokens: TokenStream) {
        self.hoist.append_all(tokens);
    }

    pub(super) fn emit(&mut self, tokens: TokenStream) {
        self.emit.append_all(tokens);
    }

    pub(super) fn finish(self) -> TokenStream {}
}
