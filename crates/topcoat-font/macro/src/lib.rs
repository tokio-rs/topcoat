use proc_macro::TokenStream;
use quote::quote;

// #[doc = include_str!("../docs/font_face.md")]
#[proc_macro]
pub fn font_face(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_font::ast::font_face::FontFace);
    quote! { #parsed }.into()
}
