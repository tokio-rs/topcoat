use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{ExprLit, Lit};
use topcoat_core::context::Cx;
use topcoat_view::runtime::{View, ViewParts};

use crate::{
    ast::expr::Expr,
    runtime::{JsViewParts, Surrogated},
};

impl Expr {
    pub(super) fn expr_lit(
        lit: &ExprLit,
        rust: &mut TokenStream,
        js: &mut String,
    ) -> syn::Result<()> {
        let mut parts = ViewParts::new();

        match &lit.lit {
            Lit::Int(inner) => {
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                let value: i32 = inner.base10_parse()?;
                value.into_surrogate().to_view_parts(&mut parts);
            }
            Lit::Float(inner) => {
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                let value: f64 = inner.base10_parse()?;
                value.into_surrogate().to_view_parts(&mut parts);
            }
            Lit::Bool(inner) => {
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                // let value = inner.value;
                // todo
                // value.into_surrogate().to_view_parts(&mut parts);
            }
            Lit::Str(inner) => {
                quote! { ::topcoat::runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                let value = inner.value();
                value.into_surrogate().to_view_parts(&mut parts);
            }
            other => return Err(syn::Error::new_spanned(other, "unsupported literal type")),
        }

        *js += &View::new(parts).render(&Cx::empty());

        Ok(())
    }
}
