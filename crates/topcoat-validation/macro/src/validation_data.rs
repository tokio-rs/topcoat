use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::shared::{is_bool, is_numeric, is_string, option_inner};

pub fn derive(input: &DeriveInput) -> TokenStream {
    let ident = &input.ident;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    input,
                    "ValidationData can only be derived for structs with named fields",
                )
                .into_compile_error();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                input,
                "ValidationData can only be derived for structs",
            )
            .into_compile_error();
        }
    };

    let arms = fields.iter().map(|field| {
        let field_ident = field.ident.as_ref().expect("named field has an ident");
        let expr = match field_value_expr(field_ident, &field.ty) {
            Ok(expr) => expr,
            Err(err) => return err.into_compile_error(),
        };
        quote! {
            stringify!(#field_ident) => #expr
        }
    });

    quote! {
        impl ::topcoat::validation::ValidationData for #ident {
            fn field(&self, name: &str) -> ::std::option::Option<::topcoat::validation::Value> {
                match name {
                    #(#arms,)*
                    _ => None,
                }
            }
        }
    }
}

fn field_value_expr(
    field_ident: &syn::Ident,
    ty: &syn::Type,
) -> Result<TokenStream, syn::Error> {
    if let Some(inner) = option_inner(ty) {
        let inner_expr = option_inner_value_expr(&inner)?;
        Ok(quote! { self.#field_ident.as_ref().map(|v| #inner_expr) })
    } else if is_string(ty) {
        Ok(quote! { Some(::topcoat::validation::Value::String(self.#field_ident.clone())) })
    } else if is_bool(ty) {
        Ok(quote! { Some(::topcoat::validation::Value::Bool(self.#field_ident)) })
    } else if is_numeric(ty) {
        Ok(quote! { Some(::topcoat::validation::Value::Number(self.#field_ident as f64)) })
    } else {
        Err(syn::Error::new_spanned(
            ty,
            "unsupported ValidationData field type: expected String, bool, a numeric primitive, or Option of those",
        ))
    }
}

fn option_inner_value_expr(ty: &syn::Type) -> Result<TokenStream, syn::Error> {
    if is_string(ty) {
        Ok(quote! { ::topcoat::validation::Value::String(v.clone()) })
    } else if is_bool(ty) {
        Ok(quote! { ::topcoat::validation::Value::Bool(*v) })
    } else if is_numeric(ty) {
        Ok(quote! { ::topcoat::validation::Value::Number(*v as f64) })
    } else {
        Err(syn::Error::new_spanned(
            ty,
            "unsupported ValidationData Option inner type: expected String, bool, or a numeric primitive",
        ))
    }
}
