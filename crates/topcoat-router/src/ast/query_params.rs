use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    DeriveInput,
    parse::{Parse, ParseStream},
};

use super::error_attr::ErrorAttr;

pub struct QueryParamsAttr {
    error: Option<ErrorAttr>,
}

impl Parse for QueryParamsAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            error: if input.is_empty() {
                None
            } else {
                Some(input.parse()?)
            },
        })
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
    #[must_use]
    pub fn new(attr: QueryParamsAttr, item: QueryParamsItem) -> Self {
        Self(attr, item)
    }

    /// Parses a `query_params` attribute and item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a
    /// [`QueryParamsAttr`] or [`QueryParamsItem`].
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for QueryParams {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let ident = &item.ident;

        let (error_ty, map_err) = match &self.0.error {
            Some(error) => (
                error.ty(),
                error.map_err(quote! {
                    |error| ::topcoat::router::bad_request_at(
                        error.path(),
                        format!("invalid query value: {}", error.inner()),
                    )
                }),
            ),
            None => (
                quote! { &'__cx ::topcoat::router::QueryParamsError },
                quote! {},
            ),
        };

        quote! {
            #[derive(::topcoat::internal::serde::Deserialize)]
            #[serde(crate = "::topcoat::internal::serde")]
            #item

            impl ::topcoat::router::QueryParams for #ident {
                type Output<'__cx> = ::core::result::Result<&'__cx Self, #error_ty>;

                fn query_params(
                    cx: &::topcoat::context::Cx,
                    _: ::topcoat::router::QueryParamsSealed,
                ) -> Self::Output<'_> {
                    #[::topcoat::context::memoize]
                    fn parse(cx: &::topcoat::context::Cx) -> ::core::result::Result<#ident, ::topcoat::router::QueryParamsError> {
                        ::topcoat::router::parse_query_params(cx)
                    }
                    parse(cx)#map_err
                }
            }
        }
        .to_tokens(tokens);
    }
}
