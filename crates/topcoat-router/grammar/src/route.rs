use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    ItemFn, LitStr,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};
use topcoat_core_grammar::paths::{topcoat_context, topcoat_inventory, topcoat_router};

use super::handler_args::{HandlerArgs, request_ident};
use super::method::Methods;

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
        let mut item = self.1.item.clone();
        item.sig.generics.params.insert(0, parse_quote! { '__cx });
        item.sig
            .inputs
            .insert(0, parse_quote! { __cx: &'__cx #topcoat_context::Cx });
        let ident = &item.sig.ident;
        let args = self.1.args.call_args();
        let parse_request = self.1.args.request().map(|request_ty| {
            let request_ident = request_ident();
            quote_spanned! {request_ty.span()=>
                let #request_ident = <#request_ty as #topcoat_router::FromRequest>::from_request(cx, body).await?;
            }
        });

        let render = quote! {
            |cx, body| {
                #[allow(clippy::unused_async)]
                #item
                Box::pin(async move {
                    #parse_request
                    #topcoat_router::IntoResponse::into_response(#ident(cx, #(#args),*).await?, cx)
                })
            }
        };

        let methods = &attr.methods;
        if let Some(path) = attr.path.as_ref() {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::RouteFn = #topcoat_router::RouteFn::const_new(
                    #methods,
                    ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#path)),
                    #render,
                );
            }
        } else {
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: #topcoat_router::ModuleRouteFn = #topcoat_router::ModuleRouteFn::new(
                    #methods,
                    module_path!(),
                    #render,
                );
            }
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { #topcoat_inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
