use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, Ident, ItemFn, LitStr, Pat,
    parse::{Parse, ParseStream},
    parse_quote,
};

pub struct LayoutAttr {
    path: Option<LitStr>,
}

impl Parse for LayoutAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            path: input.peek(LitStr).then(|| input.parse()).transpose()?,
        })
    }
}

pub struct LayoutItem {
    item: ItemFn,
    args: Vec<Ident>,
}

impl Parse for LayoutItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        let mut args = Vec::new();
        let mut has_slot = false;
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    return Err(syn::Error::new_spanned(
                        r,
                        "layout functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(pi) => {
                        args.push(pi.ident.clone());
                        if pi.ident == "slot" {
                            has_slot = true;
                        }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "layout functions only accept a `slot: Slot<'_>` and an optional `cx: &Cx` parameter",
                        ));
                    }
                },
            }
        }
        if !has_slot {
            return Err(syn::Error::new_spanned(
                &item.sig,
                "layout functions must take a `slot: Slot<'_>` parameter",
            ));
        }
        Ok(Self { item, args })
    }
}

pub struct Layout(LayoutAttr, LayoutItem);

impl Layout {
    pub fn new(attr: LayoutAttr, item: LayoutItem) -> Self {
        Self(attr, item)
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for Layout {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let mut item = self.1.item.clone();
        item.sig.generics.params.insert(0, parse_quote! { '__cx });
        item.sig
            .inputs
            .insert(0, parse_quote! { __cx: &'__cx ::topcoat::context::Cx });
        let ident = &item.sig.ident;
        let args = &self.1.args;

        let render_body = quote! {
            #item
            Box::pin(#ident(cx, #(#args),*))
        };

        let (trait_path, trait_impl) = match attr.path.as_ref() {
            Some(path) => (
                quote! { ::topcoat::router::Layout },
                quote! {
                    impl ::topcoat::router::Layout for #ident {
                        fn path(&self) -> &::topcoat::router::Path {
                            ::topcoat::router::Path::new(#path)
                        }
                        fn render<'__a>(
                            &self,
                            cx: &'__a ::topcoat::context::Cx,
                            slot: ::topcoat::router::Slot<'__a>,
                        ) -> ::topcoat::router::LayoutRenderFuture<'__a> {
                            #render_body
                        }
                    }
                },
            ),
            None => (
                quote! { ::topcoat::router::ModuleLayout },
                quote! {
                    impl ::topcoat::router::ModuleLayout for #ident {
                        fn module_path(&self) -> &'static str {
                            module_path!()
                        }
                        fn render<'__a>(
                            &self,
                            cx: &'__a ::topcoat::context::Cx,
                            slot: ::topcoat::router::Slot<'__a>,
                        ) -> ::topcoat::router::LayoutRenderFuture<'__a> {
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
