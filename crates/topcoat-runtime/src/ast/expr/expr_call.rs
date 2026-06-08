use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprCall, PathArguments, PathSegment};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_call(
        call: &ExprCall,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let segment = match &*call.func {
            SynExpr::Path(path)
                if path.qself.is_none()
                    && path.path.leading_colon.is_none()
                    && path.path.segments.len() == 1 =>
            {
                path.path.segments.first()
            }
            _ => None,
        };
        let segment = segment
            .ok_or_else(|| syn::Error::new_spanned(&call.func, "unsupported call expression"))?;
        let PathSegment { ident, arguments } = segment;
        if matches!(arguments, PathArguments::Parenthesized(_)) {
            return Err(syn::Error::new_spanned(
                arguments,
                "parenthesized generic arguments are not supported",
            ));
        }

        let (cx_method, rust_ctor) = match ident.to_string().as_str() {
            "Some" => (
                "some",
                constructor_path(
                    quote! { ::topcoat::runtime::Option },
                    arguments,
                    quote! { some },
                ),
            ),
            "Ok" => (
                "ok",
                constructor_path(
                    quote! { ::topcoat::runtime::Result },
                    arguments,
                    quote! { from_ok },
                ),
            ),
            "Err" => (
                "err",
                constructor_path(
                    quote! { ::topcoat::runtime::Result },
                    arguments,
                    quote! { from_err },
                ),
            ),
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

fn constructor_path(
    ty: TokenStream,
    arguments: &PathArguments,
    method: TokenStream,
) -> TokenStream {
    match arguments {
        PathArguments::None => quote! { #ty::#method },
        PathArguments::AngleBracketed(arguments) => quote! { #ty #arguments ::#method },
        PathArguments::Parenthesized(_) => unreachable!("rejected by caller"),
    }
}
