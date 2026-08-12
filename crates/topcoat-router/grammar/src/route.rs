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
    common::{HandlerArgs, request_ident},
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
        let vis = &self.1.item.vis;
        let docs = self
            .1
            .item
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("doc"));
        let mut item = self.1.item.clone();
        item.vis = Visibility::Inherited;
        item.sig.generics.params.insert(0, parse_quote! { '__cx });
        item.sig
            .inputs
            .insert(0, parse_quote! { __cx: &'__cx #topcoat_context::Cx });
        let ident = &item.sig.ident;
        let args = self.1.args.call_args();
        let parse_request = self.1.args.request().map(|request_ty| {
            let request_ident = request_ident();
            quote_spanned! {request_ty.span()=>
                let #request_ident = <#request_ty as #topcoat_router::request::FromRequest>::from_request(cx, body).await?;
            }
        });

        let render = quote! {
            |cx, body| {
                #[allow(clippy::unused_async)]
                #item
                Box::pin(async move {
                    #parse_request
                    #topcoat_router::response::IntoResponse::into_response(#ident(cx, #(#args),*).await?, cx)
                })
            }
        };

        let methods = &attr.methods;
        if let Some(path) = attr.path.as_ref() {
            quote! {
                #(#docs)*
                #[allow(non_upper_case_globals)]
                #vis const #ident: #topcoat_router::RouteFn = #topcoat_router::RouteFn::const_new(
                    #methods,
                    ::std::borrow::Cow::Borrowed(#topcoat_router::Path::new(#path)),
                    #render,
                );
            }
        } else {
            quote! {
                #(#docs)*
                #[allow(non_upper_case_globals)]
                #vis const #ident: #topcoat_router::ModuleRouteFn = #topcoat_router::ModuleRouteFn::new(
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
