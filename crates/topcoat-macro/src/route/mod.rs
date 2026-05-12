use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    FnArg, Ident, ItemFn, LitStr, Pat,
    parse::{Parse, ParseStream},
    parse_quote,
};

pub struct RouteAttr {
    method: Option<Ident>,
    path: Option<LitStr>,
}

impl Parse for RouteAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            method: input.peek(Ident).then(|| input.parse()).transpose()?,
            path: input.peek(LitStr).then(|| input.parse()).transpose()?,
        })
    }
}

pub struct RouteItem {
    item: ItemFn,
    args: Vec<Ident>,
}

impl Parse for RouteItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        let mut args = Vec::new();
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    return Err(syn::Error::new_spanned(
                        r,
                        "route functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(pi) => {
                        args.push(pi.ident.clone());
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "route functions only accept an optional `cx: &Cx` parameter",
                        ));
                    }
                },
            }
        }
        Ok(Self { item, args })
    }
}

pub struct Route(RouteAttr, RouteItem);

impl Route {
    pub fn new(attr: RouteAttr, item: RouteItem) -> Self {
        Self(attr, item)
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
        let args = &self.1.args;

        let default_method = Ident::new("GET", Span::call_site());
        let method = self.0.method.as_ref().unwrap_or(&default_method);

        let render = quote! {
            |cx, body| {
                #item
                Box::pin(#ident(cx, #(#args),*))
            }
        };

        match attr.path.as_ref() {
            Some(path) => quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::Route = ::topcoat::router::Route::new(
                    ::topcoat::router::Method::#method,
                    #path,
                    #render,
                );
            },
            None => quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::ModuleRoute = ::topcoat::router::ModuleRoute::new(
                    ::topcoat::router::Method::#method,
                    module_path!(),
                    #render,
                );
            },
        }
        .to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
