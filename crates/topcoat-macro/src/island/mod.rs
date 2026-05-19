mod attr;
mod item;

pub use attr::*;
pub use item::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use crate::ast::island::{IslandAttr, IslandItem};

pub struct Island {
    _attr: IslandAttr,
    item: IslandItem,
}

impl Island {
    pub fn new(attr: IslandAttr, item: IslandItem) -> Self {
        Self { _attr: attr, item }
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Island {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let mut item = self.item.item().clone();
        let ident = &item.sig.ident;

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: ::topcoat::view::Island = ::topcoat::view::Island::new(
                #render,
            );
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
