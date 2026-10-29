use std::fmt::Write;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{ExprStruct, Member, ext::IdentExt, spanned::Spanned};
use topcoat_core_grammar::paths::topcoat_runtime;

use super::js::Js;
use crate::expr::{Expr, name_resolver::NameResolver};

impl Expr {
    /// Generates Rust and JavaScript for a record's struct literal.
    ///
    /// The Rust output constructs the original struct from the fields' real
    /// values. This lets the compiler enforce its field names, types, and
    /// visibility rules. A base given with `..base` supplies the fields that
    /// the literal omits.
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
        if let (Some(dot2_token), None) = (&expr.dot2_token, &expr.rest) {
            return Err(syn::Error::new_spanned(
                quote! { #dot2_token },
                "struct update syntax requires a base expression",
            ));
        }

        // With a base, the fields are passed first so that, as in Rust, they
        // are evaluated before it.
        js.push_str(if expr.rest.is_some() {
            "cx.recordWith({"
        } else {
            "cx.record({"
        });
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
        js.push_str("}");

        // The compiler checks that the base has the record's type, so a
        // borrowed record is rejected and must be cloned first.
        let base = match &expr.rest {
            Some(base) => {
                js.push_str(", ");
                let mut value = TokenStream::new();
                Self::dispatch(base, &mut value, js, names)?;
                quote_spanned! {base.span()=> ..#topcoat_runtime::Surrogate::into_real(#value) }
            }
            None => TokenStream::new(),
        };
        js.push(')');

        let path = &expr.path;
        quote! {
            #topcoat_runtime::Surrogated::into_surrogate(#path { #(#fields,)* #base })
        }
        .to_tokens(rust);
        Ok(())
    }
}
