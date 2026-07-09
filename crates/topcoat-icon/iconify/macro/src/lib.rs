use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/include.md")]
#[proc_macro]
pub fn include(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_icon_iconify::ast::Include);
    quote! { #parsed }.into()
}

#[doc = include_str!("../docs/iconify_icon.md")]
#[proc_macro]
pub fn iconify_icon(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_icon_iconify::ast::IconifyIcon);
    quote! { #parsed }.into()
}
