#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;
use quote::quote;

#[doc = include_str!("../docs/view.md")]
#[proc_macro]
pub fn view(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view_grammar::view::View);
    quote! { #parsed }.into()
}

#[doc = include_str!("../docs/attributes.md")]
#[proc_macro]
pub fn attributes(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view_grammar::attributes::Attributes);
    quote! { #parsed }.into()
}

#[doc = include_str!("../docs/class.md")]
#[proc_macro]
pub fn class(tokens: TokenStream) -> TokenStream {
    let parsed = syn::parse_macro_input!(tokens as topcoat_view_grammar::class::Class);
    quote! { #parsed }.into()
}

#[doc = include_str!("../docs/component.md")]
#[proc_macro_attribute]
pub fn component(attr: TokenStream, item: TokenStream) -> TokenStream {
    match topcoat_view_grammar::component::Component::parse(attr.into(), item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

#[doc = include_str!("../docs/props.md")]
#[proc_macro_derive(Props, attributes(default, into))]
pub fn props(item: TokenStream) -> TokenStream {
    match topcoat_view_grammar::props::Props::parse(item.into()) {
        Ok(value) => quote! { #value }.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
