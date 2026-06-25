mod attr;
mod item;

pub use attr::*;
pub use item::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
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
        let inputs = &item.sig.inputs;
        let output = &item.sig.output;
        let block = &item.block;

        // Split the inputs into the optional `cx` parameter and the value
        // parameters that become shard arguments.
        let mut has_cx = false;
        let mut value_idents = Vec::new();
        let mut value_tys = Vec::new();
        for input in inputs {
            let syn::FnArg::Typed(pat_type) = input else {
                unreachable!("validated by ShardItem")
            };
            if let syn::Pat::Ident(pat_ident) = &*pat_type.pat
                && pat_ident.ident == "cx"
            {
                has_cx = true;
                continue;
            }
            let syn::Pat::Ident(pat_ident) = &*pat_type.pat else {
                unreachable!("validated by ShardItem")
            };
            value_idents.push(pat_ident.ident.clone());
            value_tys.push((*pat_type.ty).clone());
        }

        // The JavaScript source for each value parameter is bound to a fresh
        // ident in the component face so it can be collected into the scope.
        let js_idents: Vec<_> = value_idents
            .iter()
            .map(|id| format_ident!("__topcoat_js_{}", id))
            .collect();

        // Arguments forwarded to the hidden implementation: `cx` (when present)
        // followed by the value parameters.
        let call_args = has_cx
            .then(|| quote!(cx))
            .into_iter()
            .chain(value_idents.iter().map(|id| quote!(#id)));
        let call_args: Vec<_> = call_args.collect();

        // The component face takes each value parameter as an `Expr<T>`.
        let cx_param = has_cx.then(|| quote!(cx: &::topcoat::context::Cx,));
        let component_params = quote! {
            #cx_param
            #(#value_idents: ::topcoat::runtime::Expr<#value_tys>,)*
        };

        let impl_ident = format_ident!("__topcoat_shard_impl_{}", ident);
        let id = Uuid::new_v4().to_string();

        quote! {
            // The user's real body. Shared by the component's initial render and
            // the server endpoint that re-renders the shard.
            #[doc(hidden)]
            async fn #impl_ident(#inputs) #output #block

            // Component face: renders the shard inline, splitting each `Expr<T>`
            // into its evaluated value (for the initial server render) and its
            // JavaScript source (tracked by the browser).
            #[::topcoat::view::component]
            #vis async fn #ident(#component_params) -> ::topcoat::Result<::topcoat::view::View> {
                #(
                    let (#value_idents, #js_idents) = #value_idents.into_evaluated_and_js();
                )*
                let __placeholder = #impl_ident(#(#call_args),*).await?;
                let __scope = ::topcoat::runtime::ReactiveScope::new(
                    ::topcoat::runtime::ShardId::new(#id),
                    ::std::vec![#(#js_idents),*],
                    __placeholder,
                );
                ::topcoat::view::view! { (__scope) }
            }
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! {
                ::topcoat::internal::inventory::submit! {
                    ::topcoat::runtime::ErasedShard::new(
                        ::topcoat::runtime::ShardId::new(#id),
                        |cx, body| ::std::boxed::Box::pin(async move {
                            type __Surrogate =
                                <(#(#value_tys,)*) as ::topcoat::runtime::Surrogated>::Surrogate;
                            let ::topcoat::router::Json(__args) =
                                <::topcoat::router::Json<__Surrogate> as ::topcoat::router::FromRequest>
                                    ::from_request(cx, body).await?;
                            let (#(#value_idents,)*) =
                                ::topcoat::runtime::Surrogate::into_real(__args);
                            let __view = #impl_ident(#(#call_args),*).await?;
                            ::topcoat::Result::Ok(__view)
                        }),
                    )
                }
            }
            .to_tokens(tokens);
        }
    }
}
