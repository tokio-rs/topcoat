use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/view.md")]
#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view::ast::view::View);
    quote! { #parsed }.into()
}

#[doc = include_str!("../docs/attributes.md")]
#[proc_macro]
pub fn attributes(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view::ast::attributes::Attributes);
    quote! { #parsed }.into()
}

#[doc = include_str!("../docs/component.md")]
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_view::ast::component::Component::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn shard(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_view::ast::shard::Shard::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
