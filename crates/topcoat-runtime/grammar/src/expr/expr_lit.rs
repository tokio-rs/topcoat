use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{ExprLit, Lit, LitInt};
use topcoat_core_grammar::paths::topcoat_runtime;
use topcoat_runtime::Surrogated;

use super::js::Js;
use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    pub(super) fn expr_lit(
        lit: &ExprLit,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        match &lit.lit {
            Lit::Int(inner) => Self::integer_literal(inner, false, rust, js, names)?,
            Lit::Float(inner) => {
                quote! { #topcoat_runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                let value: f64 = inner.base10_parse()?;
                push_js_surrogate(js, &value.into_surrogate())?;
            }
            Lit::Bool(inner) => {
                quote! { #topcoat_runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                push_js_surrogate(js, &inner.value.into_surrogate())?;
            }
            Lit::Str(inner) => {
                quote! { #topcoat_runtime::Surrogated::into_surrogate(#inner) }.to_tokens(rust);
                let value = inner.value();
                push_js_surrogate(js, value.as_str().into_surrogate())?;
            }
            other => return Err(syn::Error::new_spanned(other, "unsupported literal type")),
        }

        Ok(())
    }

    pub(super) fn integer_literal(
        literal: &LitInt,
        negative: bool,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        let suffix = match literal.suffix() {
            "" => "usize",
            suffix @ ("u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32"
            | "i64" | "i128" | "isize") => suffix,
            _ => {
                return Err(syn::Error::new_spanned(
                    literal,
                    "unsupported integer suffix",
                ));
            }
        };
        if negative && suffix.starts_with('u') {
            return Err(syn::Error::new_spanned(
                literal,
                "negative integer literals require a signed suffix, such as `i32`",
            ));
        }
        let ty = syn::Ident::new(suffix, literal.span());
        let sign = negative.then(|| quote!(-));
        // Serialize on the target, not the proc-macro host: pointer widths can
        // differ when cross-compiling. Keep a signed minimum as one literal so
        // its positive magnitude is never evaluated in the signed type.
        let ident = names.capture_value(
            quote! {
                #topcoat_runtime::Expr::from({
                    let value: #ty = #sign #literal;
                    value
                })
            },
            literal.span(),
        );
        ident.to_tokens(rust);
        js.expression(&ident);
        Ok(())
    }
}

fn push_js_surrogate<T>(js: &mut Js, value: &T) -> syn::Result<()>
where
    T: serde::Serialize + ?Sized,
{
    js.push_str("cx.hydrate(");
    js.push_str(&serde_json::to_string(value).map_err(|err| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("failed to serialize literal: {err}"),
        )
    })?);
    js.push(')');
    Ok(())
}
