use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    ItemFn, LitStr, ReturnType, Visibility,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};
use topcoat_core_grammar::paths::{topcoat_context, topcoat_inventory, topcoat_router};

use super::{
    common::{HandlerArg, HandlerArgs, request_ident},
    method::Methods,
};

pub struct RouteAttr {
    methods: Methods,
    path: Option<LitStr>,
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            methods: input.parse()?,
            path: input.peek(LitStr).then(|| input.parse()).transpose()?,
        })
    }
}

pub struct RouteItem {
    item: ItemFn,
    args: HandlerArgs,
}

impl Parse for RouteItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        if item.sig.asyncness.is_none() {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "route functions must be async",
            ));
        }
        if let ReturnType::Default = &item.sig.output {
            return Err(syn::Error::new(
                item.sig.fn_token.span(),
                "route functions must declare a return type",
            ));
        }
        let args = HandlerArgs::parse(&item, "route")?;
        Ok(Self { item, args })
    }
}

pub struct Route(RouteAttr, RouteItem);

impl Route {
    #[must_use]
    pub fn new(attr: RouteAttr, item: RouteItem) -> Self {
        Self(attr, item)
    }

    /// Parses a route attribute and item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a
    /// [`RouteAttr`] or [`RouteItem`], or if the item is not a valid route
    /// handler.
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Route {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let args = &self.1.args;
        let vis = &item.vis;
        let ident = &item.sig.ident;
        let output = &item.sig.output;
        let docs = item.attrs.iter().filter(|attr| attr.path().is_ident("doc"));

        // Marker: the value users register and reference. A unit struct, so
        // `#ident` stays a value usable directly in `router.route(...)`.
        let marker = quote! {
            #(#docs)*
            #[allow(non_camel_case_types)]
            #vis struct #ident;
        };

        // The user's function, re-emitted under its original name inside the
        // bridge below to keep the module namespace clean. Its own name shadows
        // the marker within its body, so bindings named after the route keep
        // working. The injected `__cx` parameter carries the ambient context
        // that `view!` bodies read.
        let mut inner = item.clone();
        inner.vis = Visibility::Inherited;
        inner.sig.generics.params.insert(0, parse_quote! { '__cx });
        inner
            .sig
            .inputs
            .insert(0, parse_quote! { __cx: &'__cx #topcoat_context::Cx });
        inner
            .attrs
            .push(parse_quote! { #[allow(clippy::unused_async)] });

        // The bridge every caller goes through: associated items are reached
        // through the type rather than lexical scope, so `#ident::handler` is
        // callable from the trait implementation below. It forwards to the
        // user's function positionally, in declared parameter order.
        let body_param = args.request().map(|ty| quote! { , body: #ty });
        let forward_args = args.iter().map(|arg| match arg {
            HandlerArg::Cx => quote! { cx },
            HandlerArg::Request(_) => quote! { body },
        });
        let handler = quote! {
            impl #ident {
                async fn handler(cx: &#topcoat_context::Cx #body_param) #output {
                    #inner

                    #ident(cx #(, #forward_args)*).await
                }
            }
        };

        // The trait implementation dispatching requests to the bridge: it
        // parses the request body (when the route takes one), calls the bridge,
        // and converts the returned value into a response. A route with an
        // explicit path is a `Route`; one without derives its path from the
        // module tree through the module router as a `ModuleRoute`.
        let parse_request = args.request().map(|request_ty| {
            let request_ident = request_ident();
            quote_spanned! {request_ty.span()=>
                let #request_ident = <#request_ty as #topcoat_router::request::FromRequest>::from_request(cx, body).await?;
            }
        });
        let request_arg = args.request().map(|_| {
            let request_ident = request_ident();
            quote! { , #request_ident }
        });
        let methods = &attr.methods;
        let id = quote! {
            fn id(&self) -> #topcoat_router::RouteId {
                *ID
            }
        };
        let methods = quote! {
            fn methods(&self) -> #topcoat_router::Methods<'_> {
                const METHODS: #topcoat_router::Methods<'static> = #methods;
                METHODS
            }
        };
        let handle = quote! {
            fn handle<'cx>(
                &'cx self,
                cx: &'cx #topcoat_context::Cx,
                body: #topcoat_router::Body,
            ) -> #topcoat_router::RouteFuture<'cx> {
                ::std::boxed::Box::pin(async move {
                    #parse_request
                    #topcoat_router::response::AsyncIntoResponse::async_into_response(
                        #ident::handler(cx #request_arg).await?,
                        cx,
                    )
                    .await
                })
            }
        };
        let (route, submit_as) = if let Some(path) = attr.path.as_ref() {
            let route = quote! {
                impl #topcoat_router::Route for #ident {
                    #id

                    #methods

                    fn path(&self) -> &#topcoat_router::Path {
                        const PATH: &#topcoat_router::Path = #topcoat_router::Path::new(#path);
                        PATH
                    }

                    #handle
                }
            };
            (route, quote! { #topcoat_router::Route })
        } else {
            let route = quote! {
                impl #topcoat_router::ModuleRoute for #ident {
                    #id

                    #methods

                    fn module_path(&self) -> &'static str {
                        ::core::module_path!()
                    }

                    #handle
                }
            };
            (route, quote! { #topcoat_router::ModuleRoute })
        };

        // href! resolves the marker to the URL path it is served at, through
        // the router that dispatched the current request. It names the marker
        // as a type, so the marker is constructed through `Default`.
        let href_target = quote! {
            impl ::core::default::Default for #ident {
                #[inline]
                fn default() -> Self {
                    Self
                }
            }

            impl #topcoat_router::HrefTarget for #ident {
                fn path<'cx>(&self, cx: &'cx #topcoat_context::Cx) -> &'cx #topcoat_router::Path {
                    match #topcoat_router::route_endpoint(cx, *ID) {
                        ::core::option::Option::Some(endpoint) => endpoint.path(),
                        ::core::option::Option::None => ::core::panic!(::core::concat!(
                            "route `",
                            ::core::stringify!(#ident),
                            "` is not registered on the router serving this request",
                        )),
                    }
                }
            }
        };

        // Discovery collects the marker erased behind its trait.
        let submit = cfg!(feature = "discover").then(|| {
            quote! { #topcoat_inventory::submit! { &#ident as &'static dyn #submit_as } }
        });

        quote! {
            #marker

            const _: () = {
                static ID: ::std::sync::LazyLock<#topcoat_router::RouteId> =
                    ::std::sync::LazyLock::new(#topcoat_router::RouteId::new);

                #handler

                #route

                #href_target

                #submit
            };
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<RouteItem>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn accepts_async_fn_with_return_type() {
        syn::parse_str::<RouteItem>("async fn ping(cx: &Cx) -> Result { todo!() }").unwrap();
    }

    #[test]
    fn rejects_non_async_fn() {
        assert!(parse_err("fn ping() -> Result { todo!() }").contains("must be async"));
    }

    #[test]
    fn rejects_missing_return_type() {
        assert!(parse_err("async fn ping() {}").contains("must declare a return type"));
    }

    #[test]
    fn rejects_self_receiver() {
        let err = parse_err("async fn ping(&self) -> Result { todo!() }");
        assert!(err.contains("route functions cannot take a `self` receiver"));
    }
}
