use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    ItemFn, LitStr,
    parse::{Parse, ParseStream},
    parse_quote,
};

use super::handler_args::{HandlerArgs, request_ident};

pub struct PageAttr {
    path: Option<LitStr>,
}

impl Parse for PageAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.peek(LitStr).then(|| input.parse()).transpose()?,
        })
    }
}

pub struct PageItem {
    item: ItemFn,
    args: HandlerArgs,
}

impl Parse for PageItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        let args = HandlerArgs::parse(&item, "page")?;
        Ok(Self { item, args })
    }
}

pub struct Page(PageAttr, PageItem);

impl Page {
    pub fn new(attr: PageAttr, item: PageItem) -> Self {
        Self(attr, item)
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Page {
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
            quote! {
                let #request_ident = <#request_ty as ::topcoat::router::FromRequest>::from_request(cx, body).await?;
            }
        });

        let render_body = quote! {
            #item
            Box::pin(async move {
                #parse_request
                #ident(cx, #(#args),*).await
            })
        };

        let (trait_path, trait_impl) = match attr.path.as_ref() {
            Some(path) => (
                quote! { ::topcoat::router::Page },
                quote! {
                    impl ::topcoat::router::Page for #ident {
                        fn path(&self) -> &::topcoat::router::Path {
                            ::topcoat::router::Path::new(#path)
                        }
                        fn render<'__a>(
                            &'__a self,
                            cx: &'__a ::topcoat::context::Cx,
                            body: ::topcoat::router::Body,
                        ) -> ::topcoat::router::PageRenderFuture<'__a> {
                            #render_body
                        }
                    }
                },
            ),
            None => (
                quote! { ::topcoat::router::ModulePage },
                quote! {
                    impl ::topcoat::router::ModulePage for #ident {
                        fn module_path(&self) -> &'static str {
                            module_path!()
                        }
                        fn render<'__a>(
                            &'__a self,
                            cx: &'__a ::topcoat::context::Cx,
                            body: ::topcoat::router::Body,
                        ) -> ::topcoat::router::PageRenderFuture<'__a> {
                            #render_body
                        }
                    }
                },
            ),
        };

        quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy)]
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
