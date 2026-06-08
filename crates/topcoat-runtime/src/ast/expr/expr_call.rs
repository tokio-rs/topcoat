use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprCall};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_call(
        call: &ExprCall,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let ident = match &*call.func {
            SynExpr::Path(path) if path.qself.is_none() => path.path.get_ident(),
            _ => None,
        };
        let ident = ident.ok_or_else(|| {
            syn::Error::new_spanned(&call.func, "unsupported call expression")
        })?;

        let (cx_method, rust_ctor) = match ident.to_string().as_str() {
            "Some" => ("some", quote! { ::topcoat::runtime::Option::some }),
            "Ok" => ("ok", quote! { ::topcoat::runtime::Result::ok }),
            "Err" => ("err", quote! { ::topcoat::runtime::Result::err }),
            _ => {
                return Err(syn::Error::new_spanned(
                    &call.func,
                    "unsupported call expression",
                ));
            }
        };

        if call.args.len() != 1 {
            return Err(syn::Error::new_spanned(
                call,
                format!("`{ident}(...)` takes exactly one argument"),
            ));
        }
        let arg = call.args.first().unwrap();

        js.push_str("cx.");
        js.push_str(cx_method);
        js.push('(');
        let mut arg_rust = TokenStream::new();
        Self::dispatch(arg, &mut arg_rust, js, names)?;
        js.push(')');

        quote! { #rust_ctor(#arg_rust) }.to_tokens(rust);
        Ok(())
    }
}
