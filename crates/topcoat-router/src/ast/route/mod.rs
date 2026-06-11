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
    pub fn new(attr: RouteAttr, item: RouteItem) -> Self {
        Self(attr, item)
    }

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

        let method = &attr.method;
        let handle_body = quote! {
            #item
            Box::pin(async move {
                #parse_request
                ::topcoat::router::IntoResponse::into_response(#ident(cx, #(#args),*).await?)
            })
        };

        let (trait_path, trait_impl) = match attr.path.as_ref() {
            Some(path) => (
                quote! { ::topcoat::router::Route },
                quote! {
                    impl ::topcoat::router::Route for #ident {
                        fn method(&self) -> ::topcoat::router::Method {
                            ::topcoat::router::Method::#method
                        }
                        fn path(&self) -> &::topcoat::router::Path {
                            ::topcoat::router::Path::new(#path)
                        }
                        fn handle<'__a>(
                            &'__a self,
                            cx: &'__a ::topcoat::context::Cx,
                            body: ::topcoat::router::Body,
                        ) -> ::topcoat::router::RouteHandlerFuture<'__a> {
                            #handle_body
                        }
                    }
                },
            ),
            None => (
                quote! { ::topcoat::router::ModuleRoute },
                quote! {
                    impl ::topcoat::router::ModuleRoute for #ident {
                        fn method(&self) -> ::topcoat::router::Method {
                            ::topcoat::router::Method::#method
                        }
                        fn module_path(&self) -> &'static str {
                            module_path!()
                        }
                        fn handle<'__a>(
                            &'__a self,
                            cx: &'__a ::topcoat::context::Cx,
                            body: ::topcoat::router::Body,
                        ) -> ::topcoat::router::RouteHandlerFuture<'__a> {
                            #handle_body
                        }
                    }
                },
            ),
        };

        quote! {
            #[allow(non_camel_case_types)]
            #[derive(Clone, Copy)]
            struct #ident;
            #trait_impl
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! {
                ::topcoat::internal::inventory::submit! { &#ident as &'static dyn #trait_path }
            }
            .to_tokens(tokens);
        }
    }
}
