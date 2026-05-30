mod attr;
mod item;

pub use attr::*;
pub use item::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{FnArg, Pat, ReturnType, Type};
use uuid::Uuid;

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

        let return_ty = match &item.sig.output {
            ReturnType::Type(_, ty) => ty,
            ReturnType::Default => unreachable!(),
        };

        let id = Uuid::new_v4().to_string();

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: ::topcoat::runtime::Island<
                (#(#signal_types,)*),
                <#return_ty as ::topcoat::internal::ResultExt>::E,
            > = ::topcoat::runtime::Island::new(
                ::topcoat::runtime::IslandId::new(#id),
                |cx, signals| {
                    #item

                    Box::pin(#ident(#(#call_args),*))
                },
            );
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! {
                ::topcoat::internal::inventory::submit! {
                    &#ident as &'static dyn ::topcoat::runtime::DynIsland
                }
            }
            .to_tokens(tokens);
        }
    }
}
