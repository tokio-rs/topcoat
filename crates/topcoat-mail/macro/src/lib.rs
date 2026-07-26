#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;
use quote::quote;

#[proc_macro]
pub fn mail(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_mail_grammar::mail::Mail);
    quote! { #parsed }.into()
}
