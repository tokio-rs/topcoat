#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;
use quote::quote;
use topcoat_core_grammar::expand::item_with_error;

#[doc = include_str!("../docs/memoize.md")]
#[proc_macro_attribute]
pub fn memoize(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_core_grammar::memoize::Memoize::parse(attr.into(), item.clone().into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => item_with_error(&error, item.into(), &[]).into(),
    }
}
