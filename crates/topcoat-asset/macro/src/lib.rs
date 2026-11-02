#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/asset.md")]
#[proc_macro]
pub fn asset(tokens: TokenStream) -> TokenStream {
    let asset = syn::parse_macro_input!(tokens as topcoat_asset_grammar::asset::Asset);
    quote! { #asset }.into()
}
