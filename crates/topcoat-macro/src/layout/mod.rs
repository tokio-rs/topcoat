use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    ItemFn,
    parse::{Parse, ParseStream},
};

pub struct LayoutAttr {}

impl Parse for LayoutAttr {
    fn parse(_: ParseStream) -> syn::Result<Self> {
        Ok(Self {})
    }
}

pub struct LayoutItem {
    item: ItemFn,
}

impl Parse for LayoutItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            item: input.parse()?,
        })
    }
}

pub struct Layout(LayoutAttr, LayoutItem);

impl Layout {
    pub fn new(attr: LayoutAttr, item: LayoutItem) -> Self {
        Self(attr, item)
    }
}

impl ToTokens for Layout {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let ident = &item.sig.ident;

        quote! {
            #[allow(non_upper_case_globals)]
            const #ident: ::topcoat::router::layout::Layout = ::topcoat::router::layout::Layout::new(
                file!(),
                "",
                |page| {
                    #item
                    Box::pin(#ident(page))
                }
            );
        }
        .to_tokens(tokens);
    }
}
