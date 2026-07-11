use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/fontsource_font_face.md")]
#[proc_macro]
pub fn fontsource_font_face(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(
        tokens as topcoat_font_fontsource_grammar::font_face::FontsourceFontFace
    );
    quote! { #parsed }.into()
}

#[doc = include_str!("../docs/fontsource_font.md")]
#[proc_macro]
pub fn fontsource_font(tokens: TokenStream) -> TokenStream {
    let parsed =
        syn::parse_macro_input!(tokens as topcoat_font_fontsource_grammar::font::FontsourceFont);
    quote! { #parsed }.into()
}
