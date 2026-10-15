#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "fluent")]
use proc_macro::TokenStream;
#[cfg(feature = "fluent")]
use quote::quote;

#[cfg(feature = "fluent")]
#[doc = include_str!("../docs/fluent/messages.md")]
#[proc_macro]
pub fn messages(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_l10n_grammar::fluent::Messages);
    quote! { #parsed }.into()
}
