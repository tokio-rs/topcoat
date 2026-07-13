#[cfg(feature = "iconify")]
use proc_macro::TokenStream;
#[cfg(feature = "iconify")]
use quote::quote;

#[cfg(feature = "iconify")]
#[doc = include_str!("../docs/include.md")]
#[proc_macro]
pub fn include(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_icon_grammar::iconify::Include);
    quote! { #parsed }.into()
}

#[cfg(feature = "iconify")]
#[doc = include_str!("../docs/iconify_icon.md")]
#[proc_macro]
pub fn iconify_icon(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_icon_grammar::iconify::IconifyIcon);
    quote! { #parsed }.into()
}
