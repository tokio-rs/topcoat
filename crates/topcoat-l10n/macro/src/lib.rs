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
#[doc(hidden)]
#[proc_macro]
pub fn __t(tokens: TokenStream) -> TokenStream {
    let call = syn::parse_macro_input!(tokens as topcoat_l10n_grammar::fluent::TranslateCall);
    quote::quote! { #call }.into()
}
