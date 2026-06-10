use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    ItemFn,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};

use super::handler_args::request_ident;

pub struct ActionAttr {}

impl Parse for ActionAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

pub struct ActionItem {
    item: ItemFn,
}

impl Parse for ActionItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            item: input.parse()?,
        })
    }
}

pub struct Action(ActionAttr, ActionItem);

impl Action {
    pub fn new(attr: ActionAttr, item: ActionItem) -> Self {
        Self(attr, item)
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Action {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let ident = &item.sig.ident;

        let id = uuid::Uuid::new_v4().to_string();

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: ::topcoat::router::Action = ::topcoat::router::Action::new(
                ::topcoat::router::ActionId::new(#id),
            );
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
