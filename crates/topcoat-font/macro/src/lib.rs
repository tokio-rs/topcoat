use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/font_face.md")]
#[proc_macro]
pub fn font_face(tokens: TokenStream) -> TokenStream {
    let face = syn::parse_macro_input!(tokens as topcoat_font::ast::font_face::FontFace);
    quote! { #face }.into()
}

#[doc = include_str!("../docs/font.md")]
#[proc_macro]
pub fn font(tokens: TokenStream) -> TokenStream {
    let font = syn::parse_macro_input!(tokens as topcoat_font::ast::font::Font);
    quote! { #font }.into()
}
