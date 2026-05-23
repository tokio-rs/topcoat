use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Ident;

use super::Expr;

/// A `receiver.method()` call. Only zero-argument methods are supported for
/// now. Emits an accessor closure alongside the method name so rustc resolves
/// the return type from the receiver's real type, and the server-side `eval`
/// can run the method.
pub struct ExprMethodCall {
    receiver: Box<Expr>,
    method: Ident,
}

impl ExprMethodCall {
    pub fn new(receiver: Expr, method: Ident) -> Self {
        Self {
            receiver: Box::new(receiver),
            method,
        }
    }
}

impl ToTokens for ExprMethodCall {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let receiver = &self.receiver;
        let method_str = self.method.to_string();
        let method_ident = &self.method;
        quote! {
            ::topcoat::runtime::ExprMethodCall::new(
                #receiver,
                #method_str,
                |__receiver| __receiver.#method_ident(),
            )
        }
        .to_tokens(tokens);
    }
}
