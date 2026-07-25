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

/// A parsed `#[shard] async fn ...`.
pub struct Shard {
    attr: ShardAttr,
    item: ShardItem,
}

impl Shard {
    #[must_use]
    pub fn new(attr: ShardAttr, item: ShardItem) -> Self {
        Self { attr, item }
    }

    /// Parses a `#[shard]` attribute and function item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse or if a
    /// WebSocket shard has an invalid signature.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        let attr: ShardAttr = syn::parse2(attr)?;
        let item: ShardItem = syn::parse2(item)?;
        if attr.transport() == ShardTransport::WebSocket {
            item.websocket_signature()?;
        }
        Ok(Self::new(attr, item))
    }

    fn to_http_tokens(&self, tokens: &mut TokenStream) {
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

        // Arguments forwarded to the hidden implementation after the implicit
        // `__cx` context: `cx` (when present) followed by the value parameters.
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
            // the server endpoint that re-renders the shard. The leading `__cx`
            // parameter makes the request context implicitly available to macros
            // in the body (like `view!`), just as inside a `#[component]`.
            #[doc(hidden)]
            async fn #impl_ident(__cx: &#topcoat_context::Cx, #inputs) #output #block

            // Component face: renders the shard inline, splitting each `Expr<T>`
            // into its evaluated value (for the initial server render) and its
            // JavaScript source (tracked by the browser).
            #[#topcoat_view_macro::component]
            #vis async fn #ident(#component_params) -> #topcoat_error::Result<#topcoat_view::View> {
                #(
                    let (#value_idents, #js_idents) = #value_idents.into_evaluated_and_js();
                )*
                let __placeholder = #impl_ident(__cx, #(#call_args),*).await?;
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
                        let __view = #impl_ident(cx, #(#call_args),*).await?;
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

        Self::submit_discovery(&erased_ident, tokens);
    }

    fn to_websocket_tokens(&self, tokens: &mut TokenStream) {
        let item = self.item.item();
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let inputs = &item.sig.inputs;
        let output = &item.sig.output;
        let block = &item.block;
        let signature = self
            .item
            .websocket_signature()
            .expect("validated while parsing Shard");
        let argument_ident = signature.argument_ident;
        let argument_ty = signature.argument_ty;
        let has_cx = inputs.iter().any(|input| {
            matches!(input, syn::FnArg::Typed(pat_type) if matches!(&*pat_type.pat, syn::Pat::Ident(pat_ident) if pat_ident.ident == "cx"))
        });

        let js_ident = format_ident!("__topcoat_js_{}", argument_ident);
        let cx_param = has_cx.then(|| quote!(cx: &#topcoat_context::Cx,));
        let component_params = quote! {
            #cx_param
            #argument_ident: #topcoat_runtime::Expr<#argument_ty>,
        };
        let impl_context_param = (!has_cx).then(|| quote!(__cx: &#topcoat_context::Cx,));
        let impl_context_setup = has_cx.then(|| quote!(let __cx = cx;));
        let ssr_context_arg = if has_cx { quote!(cx,) } else { quote!(__cx,) };

        let impl_ident = format_ident!("__topcoat_shard_impl_{}", ident);
        let erased_ident = format_ident!("__TOPCOAT_SHARD_ERASED_{}", ident);
        let id = Uuid::new_v4().to_string();

        quote! {
            #[doc(hidden)]
            async fn #impl_ident(#impl_context_param #inputs) #output {
                #impl_context_setup
                #block
            }

            #[#topcoat_view_macro::component]
            #vis async fn #ident(#component_params) -> #topcoat_error::Result<#topcoat_view::View> {
                let (#argument_ident, #js_ident) = #argument_ident.into_evaluated_and_js();
                let __receiver =
                    #topcoat_runtime::__websocket_shard_seed(#argument_ident).await?;
                let __stream = #impl_ident(#ssr_context_arg __receiver).await;
                let __placeholder =
                    #topcoat_runtime::__websocket_shard_first(__stream).await?;
                let __scope = #topcoat_runtime::ReactiveScope::new_websocket(
                    #topcoat_runtime::ShardId::new(#id),
                    ::std::vec![#js_ident],
                    __placeholder,
                );
                #topcoat_view_macro::view! { (__scope) }
            }
        }
        .to_tokens(tokens);

        quote! {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            const #erased_ident: #topcoat_runtime::ErasedShard =
                #topcoat_runtime::ErasedShard::new_websocket(
                    #topcoat_runtime::ShardId::new(#id),
                    |cx, socket| ::std::boxed::Box::pin(async move {
                        #topcoat_runtime::__run_websocket_shard(
                            cx,
                            socket,
                            move |__receiver| async move {
                                #impl_ident(cx, __receiver).await
                            },
                        )
                        .await;
                    }),
                );

            impl ::core::convert::From<#ident> for #topcoat_runtime::ErasedShard {
                fn from(_: #ident) -> Self {
                    #erased_ident
                }
            }
        }
        .to_tokens(tokens);

        Self::submit_discovery(&erased_ident, tokens);
    }

    fn submit_discovery(erased_ident: &syn::Ident, tokens: &mut TokenStream) {
        if cfg!(feature = "discover") {
            quote! {
                #topcoat_inventory::submit! { #erased_ident }
            }
            .to_tokens(tokens);
        }
    }
}

impl ToTokens for Shard {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self.attr.transport() {
            ShardTransport::Http => self.to_http_tokens(tokens),
            ShardTransport::WebSocket => self.to_websocket_tokens(tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn websocket_expansion_uses_stream_contract_helpers() {
        let shard = Shard::parse(
            quote!(ws),
            quote! {
                async fn events(
                    cx: &Cx,
                    values: tokio::sync::mpsc::Receiver<String>,
                ) -> impl futures_core::Stream<Item = Result> {
                    stream(cx, values)
                }
            },
        )
        .unwrap();
        let expanded = quote!(#shard).to_string();

        assert!(expanded.contains("__websocket_shard_seed"));
        assert!(expanded.contains("__websocket_shard_first"));
        assert!(expanded.contains("__run_websocket_shard"));
        assert!(expanded.contains("ErasedShard :: new_websocket"));
        assert!(expanded.contains("Expr < String >"));
        assert!(!expanded.contains("Expr < tokio :: sync :: mpsc :: Receiver"));
    }

    #[test]
    fn http_expansion_remains_on_the_post_render_path() {
        let shard = Shard::parse(
            TokenStream::new(),
            quote! {
                async fn events(value: String) -> Result {
                    render(value)
                }
            },
        )
        .unwrap();
        let expanded = quote!(#shard).to_string();

        assert!(expanded.contains("ErasedShard :: new"));
        assert!(!expanded.contains("new_websocket"));
        assert!(!expanded.contains("__websocket_shard"));
    }
}
