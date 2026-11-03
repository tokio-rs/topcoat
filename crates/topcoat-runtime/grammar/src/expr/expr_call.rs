use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Expr as SynExpr, ExprCall, Token, punctuated::Punctuated};
use topcoat_core_grammar::common::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_call(
        call: &ExprCall,
        rust: &mut TokenStream,
        js: &mut Js,
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
            let (constructor, js_constructor) = match segment.ident.to_string().as_str() {
                "Some" => (
                    quote! { #topcoat_runtime::OptionSurrogate #path_arguments ::some },
                    "cx.some(",
                ),
                "Ok" => (
                    quote! { #topcoat_runtime::ResultSurrogate #path_arguments ::from_ok },
                    "cx.ok(",
                ),
                "Err" => (
                    quote! { #topcoat_runtime::ResultSurrogate #path_arguments ::from_err },
                    "cx.err(",
                ),
                _ => return Self::call(call, rust, js, names),
            };
            if args.len() != 1 {
                return Err(syn::Error::new_spanned(
                    args,
                    format!("`{}` takes exactly one argument", segment.ident),
                ));
            }
            let value = &args[0];
            *js += js_constructor;
            let mut tokens = TokenStream::new();
            Self::dispatch(value, &mut tokens, js, names)?;
            *js += ")";
            quote! { #constructor(#tokens) }.to_tokens(rust);
            return Ok(());
        }

        Self::call(call, rust, js, names)
    }

    fn call(
        call: &ExprCall,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let args = &call.args;

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
        js: &mut Js,
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
