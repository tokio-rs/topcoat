#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "fluent")]
use proc_macro::TokenStream;

#[cfg(feature = "fluent")]
#[doc = include_str!("../docs/fluent/messages.md")]
#[proc_macro]
pub fn messages(tokens: TokenStream) -> TokenStream {
    let messages = syn::parse_macro_input!(tokens as topcoat_l10n_grammar::fluent::Messages);
    quote::quote! { #messages }.into()
}

#[cfg(feature = "fluent")]
#[doc = include_str!("../docs/fluent/t.md")]
#[proc_macro]
pub fn t(tokens: TokenStream) -> TokenStream {
    let translate = syn::parse_macro_input!(tokens as topcoat_l10n_grammar::fluent::Translate);
    quote::quote! { #translate }.into()
}
