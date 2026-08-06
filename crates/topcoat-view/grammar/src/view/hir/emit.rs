use proc_macro2::TokenStream;

pub trait Emit {
    fn emit(&self, emitter: &mut Emitter);
}

pub struct Emitter {
    hoist: TokenStream,
}
