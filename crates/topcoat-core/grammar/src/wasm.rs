use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericParam, Generics, Ident};

use crate::paths::topcoat_view;

/// Emits the compiler marker identifying a render body's owner.
#[must_use]
pub fn owner(kind: &str, ident: &Ident, generics: &Generics) -> TokenStream {
    let args = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Type(param) => Some(&param.ident),
            GenericParam::Const(param) => Some(&param.ident),
            GenericParam::Lifetime(_) => None,
        })
        .collect::<Vec<_>>();
    let ty = if args.is_empty() {
        quote!(#ident)
    } else {
        quote!(#ident<#(#args),*>)
    };
    if cfg!(topcoat_wasm) {
        quote!(#topcoat_view::__topcoat_wasm_owner::<#ty>(#kind, concat!(module_path!(), "::", stringify!(#ident)));)
    } else {
        TokenStream::new()
    }
}
