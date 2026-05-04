use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    FnArg, ItemFn, LitStr, Pat,
    parse::{Parse, ParseStream},
};

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
    has_cx: bool,
}

impl Parse for PageItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: ItemFn = input.parse()?;
        let mut has_cx = false;
        for arg in &item.sig.inputs {
            match arg {
                FnArg::Receiver(r) => {
                    return Err(syn::Error::new_spanned(
                        r,
                        "page functions cannot take a `self` receiver",
                    ));
                }
                FnArg::Typed(pat_type) => match &*pat_type.pat {
                    Pat::Ident(pi) if pi.ident == "cx" => {
                        has_cx = true;
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            pat_type,
                            "page functions only accept an optional `cx: &Cx` parameter",
                        ));
                    }
                },
            }
        }
        Ok(Self { item, has_cx })
    }
}

pub struct Page(PageAttr, PageItem);

impl Page {
    pub fn new(attr: PageAttr, item: PageItem) -> Self {
        Self(attr, item)
    }
}

impl ToTokens for Page {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let attr = &self.0;
        let item = &self.1.item;
        let ident = &item.sig.ident;

        let render = if self.1.has_cx {
            quote! {
                || Box::pin(async {
                    #item
                    ::topcoat::context::with_context(async |cx| #ident(cx).await).await
                })
            }
        } else {
            quote! {
                || {
                    #item
                    Box::pin(#ident())
                }
            }
        };

        match attr.path.as_ref() {
            Some(path) => quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::Page = ::topcoat::router::Page::new(#path, #render);
            },
            None => quote! {
                #[allow(non_upper_case_globals)]
                const #ident: ::topcoat::router::ModulePage = ::topcoat::router::ModulePage::new(module_path!(), #render);
            }
        }.to_tokens(tokens);

        if cfg!(feature = "discover") {
            quote! { ::topcoat::internal::inventory::submit! { #ident } }.to_tokens(tokens);
        }
    }
}
