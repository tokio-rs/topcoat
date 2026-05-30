use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::ExprMethodCall;

use crate::ast::expr::{Expr, name_resolver::NameResolver};

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
        write!(js, ".{method}(").unwrap();

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
