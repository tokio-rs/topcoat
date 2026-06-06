use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    DeriveInput,
    parse::{Parse, ParseStream},
};

pub struct QueryParamsAttr;

impl Parse for QueryParamsAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self)
    }
}

pub struct QueryParamsItem {
    item: DeriveInput,
}

impl Parse for QueryParamsItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            item: input.parse()?,
        })
    }
}

pub struct QueryParams(QueryParamsAttr, QueryParamsItem);

impl QueryParams {
    pub fn new(attr: QueryParamsAttr, item: QueryParamsItem) -> Self {
        Self(attr, item)
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for QueryParams {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let ident = &item.ident;

        quote! {
            #[derive(::topcoat::internal::serde::Deserialize)]
            #[serde(crate = "::topcoat::internal::serde")]
            #item

            impl #ident {
                fn of<'__cx>(
                    cx: &'__cx ::topcoat::context::Cx,
                ) -> ::core::result::Result<&'__cx Self, &'__cx ::topcoat::internal::serde_urlencoded::de::Error> {
                    #[::topcoat::context::memoize]
                    fn parse(cx: &::topcoat::context::Cx) -> ::core::result::Result<#ident, ::topcoat::internal::serde_urlencoded::de::Error> {
                        ::topcoat::internal::serde_urlencoded::from_str(
                            ::topcoat::router::uri(cx).path_and_query().map(|pq| pq.query().unwrap_or("")).unwrap_or("")
                        )
                    }
                    parse(cx)
                }
            }
        }
        .to_tokens(tokens);
    }
}
