use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/font_face.md")]
#[proc_macro]
pub fn font_face(tokens: TokenStream) -> TokenStream {
    let face = syn::parse_macro_input!(tokens as topcoat_font_grammar::font_face::FontFace);
    quote! { #face }.into()
}

#[doc = include_str!("../docs/font.md")]
#[proc_macro]
pub fn font(tokens: TokenStream) -> TokenStream {
    let font = syn::parse_macro_input!(tokens as topcoat_font_grammar::font::Font);
    quote! { #font }.into()
}

#[cfg(feature = "fontsource")]
#[doc = include_str!("../docs/fontsource_font_face.md")]
#[proc_macro]
pub fn fontsource_font_face(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(
        tokens as topcoat_font_grammar::fontsource::font_face::FontsourceFontFace
    );
    quote! { #parsed }.into()
}

#[cfg(feature = "fontsource")]
#[doc = include_str!("../docs/fontsource_font.md")]
#[proc_macro]
pub fn fontsource_font(tokens: TokenStream) -> TokenStream {
    let parsed =
        syn::parse_macro_input!(tokens as topcoat_font_grammar::fontsource::font::FontsourceFont);
    quote! { #parsed }.into()
}
