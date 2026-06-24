use proc_macro2::TokenStream;
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Ident, ItemFn, LitStr,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};

use super::handler_args::{HandlerArgs, request_ident};

pub struct RouteAttr {
    method: Ident,
    path: Option<LitStr>,
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            method: input
                .peek(Ident)
                .then(|| input.parse())
                .transpose()?
                .ok_or_else(|| {
                    syn::Error::new(
                        input.span(),
                        "route attributes must start with an HTTP method",
                    )
                })?,
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
            .insert(0, parse_quote! { __cx: &'__cx ::topcoat::context::Cx });
        let ident = &item.sig.ident;
        let args = self.1.args.call_args();
        let parse_request = self.1.args.request().map(|request| {
            let request_ident = request_ident();
            let request_ty = &request.ty;
            quote_spanned! {request_ty.span()=>
                let #request_ident = <#request_ty as ::topcoat::router::FromRequest>::from_request(cx, body).await?;
            }
        });

        let render = quote! {
            |cx, body| {
                #[allow(clippy::unused_async)]
                #item
                Box::pin(async move {
                    #parse_request
                    ::topcoat::router::IntoResponse::into_response(#ident(cx, #(#args),*).await?)
                })
            }
        };

        if let Some(path) = attr.path.as_ref() {
            let method = &attr.method;
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::RouteFn = ::topcoat::router::RouteFn::new(
                    ::topcoat::router::Method::#method,
                    ::std::borrow::Cow::Borrowed(::topcoat::router::Path::new(#path)),
                    #render,
                );
            }
        } else {
            let method = &attr.method;
            quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::ModuleRouteFn = ::topcoat::router::ModuleRouteFn::new(
                    ::topcoat::router::Method::#method,
                    module_path!(),
                    #render,
                );
            }
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
