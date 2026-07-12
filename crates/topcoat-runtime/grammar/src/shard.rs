mod attr;
mod item;

pub use attr::*;
pub use item::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use topcoat_core_grammar::paths::{
    topcoat_context, topcoat_error, topcoat_inventory, topcoat_router, topcoat_runtime,
    topcoat_view, topcoat_view_macro,
};
use uuid::Uuid;

use crate::shard::{ShardAttr, ShardItem};

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
        let cx_param = has_cx.then(|| quote!(cx: &#topcoat_context::Cx,));
        // Bound to a local because it is interpolated inside the `#(...)*`
        // repetition below, where a bare `#topcoat_runtime` would expand to a
        // `let` binding that cannot shadow the imported constant.
        let expr_ty = quote!(#topcoat_runtime::Expr);
        let component_params = quote! {
            #cx_param
            #(#value_idents: #expr_ty<#value_tys>,)*
        };

        let impl_ident = format_ident!("__topcoat_shard_impl_{}", ident);
        let erased_ident = format_ident!("__TOPCOAT_SHARD_ERASED_{}", ident);
        let id = Uuid::new_v4().to_string();

        quote! {
            // The user's real body. Shared by the component's initial render and
            // the server endpoint that re-renders the shard.
            #[doc(hidden)]
            async fn #impl_ident(#inputs) #output #block

            // Component face: renders the shard inline, splitting each `Expr<T>`
            // into its evaluated value (for the initial server render) and its
            // JavaScript source (tracked by the browser).
            #[#topcoat_view_macro::component]
            #vis async fn #ident(#component_params) -> #topcoat_error::Result<#topcoat_view::View> {
                #(
                    let (#value_idents, #js_idents) = #value_idents.into_evaluated_and_js();
                )*
                let __placeholder = #impl_ident(#(#call_args),*).await?;
                let __scope = #topcoat_runtime::ReactiveScope::new(
                    #topcoat_runtime::ShardId::new(#id),
                    ::std::vec![#(#js_idents),*],
                    __placeholder,
                );
                #topcoat_view_macro::view! { (__scope) }
            }
        }
        .to_tokens(tokens);

        // The erased shard is built once in a `const` so it can be used from
        // both the `From` impl (for manual `router.shard(#ident)` registration)
        // and the discovery submission (which expands to a `static`, requiring a
        // const initializer). The marker the component face expands to is a unit
        // struct, so `#ident` is a value usable just like `router.page(...)`.
        quote! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            const #erased_ident: #topcoat_runtime::ErasedShard =
                #topcoat_runtime::ErasedShard::new(
                    #topcoat_runtime::ShardId::new(#id),
                    |cx, body| ::std::boxed::Box::pin(async move {
                        type __Surrogate =
                            <(#(#value_tys,)*) as #topcoat_runtime::Surrogated>::Surrogate;
                        let #topcoat_router::Json(__args) =
                            <#topcoat_router::Json<__Surrogate> as #topcoat_router::FromRequest>
                                ::from_request(cx, body).await?;
                        let (#(#value_idents,)*) =
                            #topcoat_runtime::Surrogate::into_real(__args);
                        let __view = #impl_ident(#(#call_args),*).await?;
                        #topcoat_error::Result::Ok(__view)
                    }),
                );

            impl ::core::convert::From<#ident> for #topcoat_runtime::ErasedShard {
                fn from(_: #ident) -> Self {
                    #erased_ident
                }
            }
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! {
                #topcoat_inventory::submit! { #erased_ident }
            }
            .to_tokens(tokens);
        }
    }
}
