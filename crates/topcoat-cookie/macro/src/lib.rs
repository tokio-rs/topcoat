#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/cookie.md")]
#[proc_macro]
pub fn cookie(tokens: TokenStream) -> TokenStream {
    let cookie = syn::parse_macro_input!(tokens as topcoat_cookie_grammar::cookie::Cookie);
    quote! { #cookie }.into()
}
