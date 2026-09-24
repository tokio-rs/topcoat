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

use crate::{
    common::EndpointPath,
    shard::{ShardAttr, ShardItem},
};

/// The path prefix for shards without an explicit path.
const SHARD_ROUTE_PREFIX: &str = "/_topcoat/runtime/shards";

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

        // Arguments forwarded to the hidden implementation after the implicit
        // `__cx` context: `cx` (when present) followed by the value parameters.
        let call_args = has_cx
            .then(|| quote!(cx))
            .into_iter()
            .chain(value_idents.iter().map(|id| quote!(#id)));
        let call_args: Vec<_> = call_args.collect();

        // The component face accepts a value or expression for each parameter.
        let cx_param = has_cx.then(|| quote!(cx: &#topcoat_context::Cx,));
        // Bound to a local because it is interpolated inside the `#(...)*`
        // repetition below, where a bare `#topcoat_runtime` would expand to a
        // `let` binding that cannot shadow the imported constant.
        let expr_ty = quote!(#topcoat_runtime::Expr);
        let component_params = quote! {
            #cx_param
            #(#[into] #value_idents: #expr_ty<#value_tys>,)*
        };

        // Marker: the value users register and reference, expanded from the
        // component face. It renders the shard inline, splitting each
        // `Expr<T>` into its evaluated value (for the initial server render)
        // and its JavaScript source (tracked by the browser). The marker
        // struct the face expands to is a unit struct, so `#ident` stays a
        // value usable directly in `router.route(...)`. The function's doc
        // comments ride along on the face, so the component expansion carries
        // them onto the marker.
        //
        // The handler polls inside its own `HoistView` on both paths, so
        // parts hoisted while the shard body runs land inside the scope
        // markers, where the browser attributes them to the shard.
        //
        // The scope marker carries the endpoint's URL, where the browser
        // posts re-render requests. The invocation's identity travels on the
        // same marker and comes back in the identity header of every
        // re-render request, where the router installs it on the context, so
        // identities derived inside the shard body match between the inline
        // render and a re-render.
        //
        // The scope stays a view rather than settling the handler's content,
        // so a live shard body keeps streaming its updates through the
        // enclosing content.
        let path = EndpointPath::resolve(self.attr.path.as_ref(), SHARD_ROUTE_PREFIX);
        let url = EndpointPath::url(&path);
        let docs = item.attrs.iter().filter(|attr| attr.path().is_ident("doc"));
        let marker = quote! {
            #(#docs)*
            #[#topcoat_view_macro::component]
            #vis async fn #ident(#component_params) -> #topcoat_error::Result<impl #topcoat_view::View> {
                let __identity = #topcoat_context::identity(__cx);
                #(
                    let (#value_idents, #js_idents) = #value_idents.into_evaluated_and_js();
                )*
                #topcoat_error::Result::Ok(#topcoat_runtime::ShardScope::new(
                    __cx,
                    __identity,
                    #url,
                    ::std::vec![#(#js_idents),*],
                    #topcoat_view::HoistView::new(#topcoat_view::internal::ThenView::new(
                        #ident::handler(__cx, #(#call_args),*),
                    )),
                ))
            }
        };

        // The user's function body, re-emitted as the marker's `handler`
        // associated function. Associated items are reached through the type
        // rather than lexical scope, so `#ident::handler` is callable from
        // the component face and the route implementation below. The leading
        // `__cx` parameter carries the ambient context that `view!` bodies
        // read.
        let handler = quote! {
            impl #ident {
                #[allow(clippy::unused_async, clippy::unused_async_trait_impl)]
                async fn handler(
                    __cx: &#topcoat_context::Cx,
                    #inputs
                ) #output #block
            }
        };

        // The route serving re-render requests at the endpoint's path. Its
        // handler deserializes the surrogate argument tuple and the signal
        // values from the request body, installs the values, forwards the
        // arguments to the handler positionally, and responds with the
        // rendered view. The view streams like a page's, so a live shard
        // body's later updates follow its first content in the response.
        let route = quote! {
            impl #topcoat_router::Route for #ident {
                fn id(&self) -> #topcoat_router::RouteId {
                    *ID
                }

                fn methods(&self) -> #topcoat_router::Methods<'_> {
                    // Avoids URL length limits for large arguments.
                    const METHODS: #topcoat_router::Methods<'static> =
                        #topcoat_router::Methods::Only(&[#topcoat_router::Method::POST]);
                    METHODS
                }

                fn path(&self) -> &#topcoat_router::Path {
                    const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                    PATH
                }

                fn handle<'cx>(
                    &'cx self,
                    cx: &'cx #topcoat_context::Cx,
                    body: #topcoat_router::Body,
                ) -> #topcoat_router::RouteFuture<'cx> {
                    ::std::boxed::Box::pin(async move {
                        type __Surrogate =
                            <(#(#value_tys,)*) as #topcoat_runtime::Surrogated>::Surrogate;
                        let #topcoat_router::content::Json(__request) =
                            <#topcoat_router::content::Json<#topcoat_runtime::ShardRequest<__Surrogate>> as #topcoat_router::request::FromRequest>
                                ::from_request(cx, body).await?;
                        let (__args, __signals) = __request.into_parts();
                        let (#(#value_idents,)*) =
                            #topcoat_runtime::Surrogate::into_real(__args);
                        // Signals created while the handler runs resume from
                        // the values the client sent.
                        let cx = cx.with(__signals);
                        // The response body outlives the handler, so the
                        // view owns a copy of the request context and
                        // drives itself in place as the outermost view of
                        // the build, whose content is self-contained.
                        let __owned = cx.clone();
                        let __view = #topcoat_view::internal::MoveView::new(async move {
                            let cx = &__owned;
                            let __view = #topcoat_view::internal::ScopeView::new(
                                #topcoat_view::HoistView::new(
                                    #topcoat_view::internal::ThenView::new(
                                        #ident::handler(cx, #(#call_args),*),
                                    ),
                                ),
                            );
                            #topcoat_view::internal::MoveView::drive(__view).await
                        });
                        #topcoat_router::response::AsyncIntoResponse::async_into_response(
                            __view, &cx,
                        )
                        .await
                    })
                }
            }
        };

        // Discovery collects the marker as a route.
        let submit = cfg!(feature = "discover").then(|| {
            quote! { #topcoat_inventory::submit! { &#ident as &'static dyn #topcoat_router::Route } }
        });

        quote! {
            #marker

            const _: () = {
                static ID: ::std::sync::LazyLock<#topcoat_router::RouteId> =
                    ::std::sync::LazyLock::new(#topcoat_router::RouteId::new);

                #handler

                #route

                #submit
            };
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use quote::quote;

    use super::*;

    #[test]
    fn doc_comments_ride_along_on_the_component_face() {
        let shard = Shard::parse(
            TokenStream::new(),
            quote! {
                /// Counts clicks.
                async fn counter(count: i64) -> Result<impl View> { todo!() }
            },
        )
        .unwrap();
        let out = shard.to_token_stream().to_string();
        let doc = out.find("Counts clicks.").expect(&out);
        let face = out.find("async fn counter").expect(&out);
        assert!(doc < face, "{out}");
    }

    #[test]
    fn a_named_path_replaces_the_default_route() {
        let shard = Shard::parse(
            quote! { "/search" },
            quote! {
                async fn counter(count: i64) -> Result<impl View> { todo!() }
            },
        )
        .unwrap();
        let out = shard.to_token_stream().to_string();
        assert!(out.contains(r#""/search""#), "{out}");
        assert!(!out.contains(SHARD_ROUTE_PREFIX), "{out}");
    }

    #[test]
    fn a_shard_without_a_path_is_served_below_the_default_prefix() {
        let shard = Shard::parse(
            TokenStream::new(),
            quote! {
                async fn counter(count: i64) -> Result<impl View> { todo!() }
            },
        )
        .unwrap();
        let out = shard.to_token_stream().to_string();
        assert!(out.contains(SHARD_ROUTE_PREFIX), "{out}");
    }

    #[test]
    fn the_scope_marker_carries_the_url_without_group_segments() {
        let shard = Shard::parse(
            quote! { "/(api)/search" },
            quote! {
                async fn counter(count: i64) -> Result<impl View> { todo!() }
            },
        )
        .unwrap();
        let out = shard.to_token_stream().to_string();
        assert!(out.contains(r#""/search""#), "{out}");
        assert!(out.contains(r#""/(api)/search""#), "{out}");
    }
}
