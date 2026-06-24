mod attr;
mod item;

pub use attr::*;
pub use item::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{FnArg, Pat, Type};
use uuid::Uuid;

use crate::ast::shard::{ShardAttr, ShardItem};

/// A parsed `#[shard] async fn ...`. Expands into a `const` of type
/// [`topcoat::runtime::Shard`] paired with a unique [`ShardId`], so the shard
/// can be invoked by id and (when `discover` is enabled) auto-registered with
/// the runtime inventory.
///
/// [`ShardId`]: ::topcoat::runtime::ShardId
pub struct Shard {
    _attr: ShardAttr,
    item: ShardItem,
}

impl Shard {
    pub fn new(attr: ShardAttr, item: ShardItem) -> Self {
        Self { _attr: attr, item }
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Shard {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = self.item.item();
        let ident = &item.sig.ident;

        let mut call_args: Vec<TokenStream> = Vec::new();
        let mut signal_types: Vec<&Type> = Vec::new();
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(pi) if pi.ident == "cx" => call_args.push(quote! { cx }),
                    _ => {
                        let idx = syn::Index::from(signal_types.len());
                        call_args.push(quote! { signals.#idx });
                        signal_types.push(&pat_type.ty);
                    }
                },
                FnArg::Receiver(_) => unreachable!(),
            }
        }

        let id = Uuid::new_v4().to_string();

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: ::topcoat::runtime::Shard<(#(#signal_types,)*)> = ::topcoat::runtime::Shard::new(
                ::topcoat::runtime::ShardId::new(#id),
                |cx, signals| {
                    #item

                    Box::pin(#ident(#(#call_args),*))
                },
            );
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
