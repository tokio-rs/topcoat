use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{ExprLit, Lit};

use crate::{ast::expr::Expr, runtime::Surrogated};

impl Expr {
    pub(super) fn expr_lit(
        lit: &ExprLit,
        rust: &mut TokenStream,
        js: &mut String,
    ) -> syn::Result<()> {
        match &lit.lit {
            Lit::Float(inner) => {
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                let value: f64 = inner.base10_parse()?;
                push_js_surrogate(js, &value.into_surrogate())?;
            }
            Lit::Bool(inner) => {
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                push_js_surrogate(js, &inner.value.into_surrogate())?;
            }
            Lit::Str(inner) => {
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                let value = inner.value();
                push_js_surrogate(js, value.as_str().into_surrogate())?;
            }
            other => return Err(syn::Error::new_spanned(other, "unsupported literal type")),
        }

        Ok(())
    }
}

fn push_js_surrogate<T>(js: &mut String, value: &T) -> syn::Result<()>
where
    T: serde::Serialize + ?Sized,
{
    js.push_str("cx.s(");
    js.push_str(&serde_json::to_string(value).map_err(|err| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("failed to serialize literal: {err}"),
        )
    })?);
    js.push(')');
    Ok(())
}
