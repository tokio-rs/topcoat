use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::fmt::Write;
use syn::{ExprLit, Lit};

use crate::{ast::expr::Expr, runtime::Interop};

impl Expr {
    pub(super) fn expr_lit(
        lit: &ExprLit,
        rust: &mut TokenStream,
        js: &mut String,
    ) -> syn::Result<()> {
        match &lit.lit {
            Lit::Float(inner) => {
                quote! { ::topcoat::runtime::Interop::into_surrogate(#inner) }.to_tokens(rust);
                let value: f64 = inner.base10_parse()?;
                value.to_js(js);
            }
            Lit::Bool(inner) => {
                quote! { ::topcoat::runtime::Interop::into_surrogate(#inner) }.to_tokens(rust);
                write!(js, "{}", inner.value).unwrap();
            }
            Lit::Str(inner) => {
                quote! { ::topcoat::runtime::Interop::into_surrogate(#inner) }.to_tokens(rust);
                js.push_str(&serde_json::to_string(&inner.value()).unwrap());
            }
            other => return Err(syn::Error::new_spanned(other, "unsupported literal type")),
        }
        Ok(())
    }
}
