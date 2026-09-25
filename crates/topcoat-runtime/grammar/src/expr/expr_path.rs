use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{ExprPath, PathArguments, PathSegment};
use topcoat_core_grammar::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{
    Expr,
    name_resolver::{NameResolver, ResolvedIdent},
};

impl Expr {
    pub(super) fn expr_path(
        path: &ExprPath,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let segment = if path.qself.is_none()
            && path.path.leading_colon.is_none()
            && path.path.segments.len() == 1
        {
            path.path.segments.first()
        } else {
            None
        };
        let segment = segment.ok_or_else(|| {
            syn::Error::new_spanned(path, "only single-identifier paths are supported")
        })?;
        let PathSegment { ident, arguments } = segment;
        if matches!(arguments, PathArguments::Parenthesized(_)) {
            return Err(syn::Error::new_spanned(
                arguments,
                "parenthesized generic arguments are not supported",
            ));
        }

        if ident == "None" {
            js.push_str("cx.none()");
            let ctor = match arguments {
                PathArguments::None => quote! { #topcoat_runtime::Option::<_>::none },
                PathArguments::AngleBracketed(arguments) => {
                    quote! { #topcoat_runtime::Option #arguments ::none }
                }
                PathArguments::Parenthesized(_) => unreachable!("rejected above"),
            };
            quote! { #ctor() }.to_tokens(rust);
            return Ok(());
        }

        if !matches!(arguments, PathArguments::None) {
            return Err(syn::Error::new_spanned(
                arguments,
                "generic arguments are only supported for `None` paths",
            ));
        }

        let resolved = names.resolve(ident);
        let rust_ident = match resolved {
            ResolvedIdent::Local {
                js_name,
                rust_ident,
            } => {
                js.push_str(&js_name);
                rust_ident
            }
            ResolvedIdent::External { rust_ident } => {
                js.expression(&rust_ident);
                rust_ident
            }
        };
        rust_ident.to_tokens(rust);
        Ok(())
    }
}
