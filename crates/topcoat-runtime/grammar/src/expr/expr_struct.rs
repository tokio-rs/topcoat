use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{ExprStruct, Member, ext::IdentExt, spanned::Spanned};
use topcoat_core_grammar::common::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    /// Generates Rust and JavaScript for a record's struct literal.
    ///
    /// The Rust output constructs the original struct from the fields' real
    /// values. This lets the compiler enforce its field names, types, and
    /// visibility rules.
    pub(super) fn expr_struct(
        expr: &ExprStruct,
        rust: &mut TokenStream,
        js: &mut Js,
        names: &mut NameResolver,
    ) -> syn::Result<()> {
        if let Some(attr) = expr.attrs.first() {
            return Err(syn::Error::new_spanned(
                attr,
                "attributes are not supported",
            ));
        }
        if let Some(qself) = &expr.qself {
            return Err(syn::Error::new_spanned(
                &qself.ty,
                "qualified struct paths are not supported",
            ));
        }
        if let Some(dot2_token) = &expr.dot2_token {
            return Err(syn::Error::new_spanned(
                quote! { #dot2_token },
                "struct update syntax is not supported",
            ));
        }

        js.push_str("cx.record({");
        let mut fields = Vec::with_capacity(expr.fields.len());
        for (index, field) in expr.fields.iter().enumerate() {
            if let Some(attr) = field.attrs.first() {
                return Err(syn::Error::new_spanned(
                    attr,
                    "attributes are not supported",
                ));
            }
            let Member::Named(name) = &field.member else {
                return Err(syn::Error::new_spanned(
                    &field.member,
                    "only named fields are supported",
                ));
            };
            if index > 0 {
                js.push_str(", ");
            }
            write!(js, "{}: ", name.unraw()).unwrap();

            let mut value = TokenStream::new();
            Self::dispatch(&field.expr, &mut value, js, names)?;
            fields.push(quote_spanned! {field.expr.span()=>
                #name: #topcoat_runtime::Surrogate::into_real(#value)
            });
        }
        js.push_str("})");

        let path = &expr.path;
        quote! {
            #topcoat_runtime::Surrogated::into_surrogate(#path { #(#fields),* })
        }
        .to_tokens(rust);
        Ok(())
    }
}
