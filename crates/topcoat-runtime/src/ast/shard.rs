mod attr;
mod item;

pub use attr::*;
pub use item::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use uuid::Uuid;

use crate::ast::shard::{ShardAttr, ShardItem};

/// A parsed `#[shard] async fn ...`.
pub struct Shard {
    _attr: ShardAttr,
    item: ShardItem,
}

impl Shard {
    #[must_use]
    pub fn new(attr: ShardAttr, item: ShardItem) -> Self {
        Self { _attr: attr, item }
    }

    /// Parses a `#[shard]` attribute and function item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a
    /// `ShardAttr` or `ShardItem`.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Shard {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = self.item.item();
        let vis = &item.vis;
        let ident = &item.sig.ident;

        let id = Uuid::new_v4().to_string();

        quote! {
            #[::topcoat::view::component]
            #vis async fn #ident() -> ::topcoat::Result<::topcoat::view::View> {
                ::topcoat::view::view! {

                }
            }
        }
        .to_tokens(tokens);

        // TODO
        // if cfg!(feature = "discover") {
        //     quote! {
        //         ::topcoat::internal::inventory::submit! {
        //             &#ident as &'static dyn ::topcoat::runtime::DynShard
        //         }
        //     }
        //     .to_tokens(tokens);
        // }
    }
}
