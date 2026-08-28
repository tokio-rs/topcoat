use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprMethodCall;

use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_method_call(
        call: &ExprMethodCall,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        if let Some(turbofish) = &call.turbofish {
            return Err(syn::Error::new_spanned(
                turbofish,
                "turbofish is not supported",
            ));
        }

        let mut receiver = TokenStream::new();
        Self::dispatch(&call.receiver, &mut receiver, js, names)?;

        let method = &call.method;

        // JavaScript treats any object with a callable `then` property as a
        // Promise-like thenable. Exposing Rust's `bool::then` under that
        // name causes boolean procedure results resolve to `undefined`.
        // To prevent this, here we add an exception for methods named `then`
        // to be defined as `then_`.
        if method == "then" {
            js.push_str(".then_(");
        } else {
            write!(js, ".{method}(").unwrap();
        }

        let mut args = Vec::with_capacity(call.args.len());
        for (i, arg) in call.args.iter().enumerate() {
            if i > 0 {
                js.push_str(", ");
            }
            let mut tokens = TokenStream::new();
            Self::dispatch(arg, &mut tokens, js, names)?;
            args.push(tokens);
        }
        js.push(')');

        quote! { (#receiver).#method(#(#args),*) }.to_tokens(rust);
        Ok(())
    }
}
