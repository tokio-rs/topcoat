use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprCall, Token, punctuated::Punctuated};

use crate::ast::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_call(
        call: &ExprCall,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let args = &call.args;
        // Special case for enum construction
        if let SynExpr::Path(path) = &*call.func
            && path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
        {
            let segment = path.path.segments.first().unwrap();
            let path_arguments = &segment.arguments;
            match segment.ident.to_string().as_str() {
                "Some" => {
                    quote! { ::topcoat::runtime::Option #path_arguments ::some }.to_tokens(rust);
                    *js += "cx.some";
                    return Self::args(args, rust, js, names);
                }
                "Ok" => {
                    quote! { ::topcoat::runtime::Result #path_arguments ::from_ok }.to_tokens(rust);
                    *js += "cx.ok";
                    return Self::args(args, rust, js, names);
                }
                "Err" => {
                    quote! { ::topcoat::runtime::Result #path_arguments ::from_err }
                        .to_tokens(rust);
                    *js += "cx.err";
                    return Self::args(args, rust, js, names);
                }
                _ => {
                    // fall through to .call(...)
                }
            }
        }

        // `.call(...)` syntax
        Self::dispatch(&call.func, rust, js, names)?;
        *js += ".call";
        quote! { .call }.to_tokens(rust);
        Self::args(args, rust, js, names)?;

        Ok(())
    }

    fn args(
        args: &Punctuated<syn::Expr, Token![,]>,
        rust: &mut TokenStream,
        js: &mut String,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let mut tokens = TokenStream::new();
        *js += "(";
        for (index, arg) in args.iter().enumerate() {
            Self::dispatch(arg, &mut tokens, js, names)?;
            if index < args.len() - 1 {
                *js += ", ";
            }
            quote! { , }.to_tokens(&mut tokens);
        }
        *js += ")";
        quote! { ((#tokens)) }.to_tokens(rust);
        Ok(())
    }
}
