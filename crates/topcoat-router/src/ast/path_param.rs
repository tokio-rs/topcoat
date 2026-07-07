use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Data, DeriveInput, Fields, Type,
    parse::{Parse, ParseStream},
};

use super::error_attr::ErrorAttr;

pub struct PathParamAttr {
    error: Option<ErrorAttr>,
}

impl Parse for PathParamAttr {
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

pub struct PathParamItem {
    item: DeriveInput,
    inner_ty: Type,
}

impl PathParamItem {
    /// Whether the parameter borrows the raw segment (a `str` inner type)
    /// rather than parsing it.
    fn borrows_raw_segment(&self) -> bool {
        matches!(
            &self.inner_ty,
            Type::Path(path) if path.qself.is_none() && path.path.is_ident("str")
        )
    }

}

impl Parse for PathParamItem {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item: DeriveInput = input.parse()?;
        let Data::Struct(data_struct) = &item.data else {
            return Err(syn::Error::new_spanned(
                &item.ident,
                "path_param can only be applied to a tuple struct with one unnamed field",
            ));
        };
        let Fields::Unnamed(unnamed) = &data_struct.fields else {
            return Err(syn::Error::new_spanned(
                &data_struct.fields,
                "path_param can only be applied to a tuple struct with one unnamed field",
            ));
        };
        if unnamed.unnamed.len() != 1 {
            return Err(syn::Error::new_spanned(
                &unnamed.unnamed,
                "path_param structs must have exactly one unnamed field",
            ));
        }
        let inner_ty = unnamed.unnamed.first().unwrap().ty.clone();
        Ok(Self { item, inner_ty })
    }
}

pub struct PathParam(PathParamAttr, PathParamItem);

impl PathParam {
    /// Combines a parsed attribute and item.
    ///
    /// # Errors
    ///
    /// Returns an error if the attribute declares `error = ...` for a `&str`
    /// parameter, which borrows the raw segment and cannot fail.
    pub fn new(attr: PathParamAttr, item: PathParamItem) -> syn::Result<Self> {
        if let Some(error) = &attr.error {
            if item.borrows_raw_segment() {
                return Err(syn::Error::new(
                    error.span(),
                    "`error` cannot be used with a `&str` path parameter, \
                     which borrows the raw segment and cannot fail",
                ));
            }
        }
        Ok(Self(attr, item))
    }

    /// Parses a `path_param` attribute and item from token streams.
    ///
    /// # Errors
    ///
    /// Returns an error if either token stream fails to parse as a
    /// [`PathParamAttr`] or [`PathParamItem`], if the item is not a tuple
    /// struct with exactly one unnamed field, or if the attribute and item
    /// disagree as described on [`PathParam::new`].
    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Self::new(syn::parse2(attr)?, syn::parse2(item)?)
    }
}

impl ToTokens for PathParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let ident = &item.ident;
        let inner_ty = &self.1.inner_ty;
        let name_string = ident.to_string().to_snake_case();
        let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

        let (output_ty, path_param_fn) = if self.1.borrows_raw_segment() {
            (
                quote! { &'__cx str },
                quote! {
                    fn path_param(
                        cx: &::topcoat::context::Cx,
                        _: ::topcoat::router::PathParamSealed,
                    ) -> Self::Output<'_> {
                        for (key, value) in ::topcoat::router::raw_path_params(cx) {
                            if key == #name_string {
                                return value;
                            }
                        }
                        panic!("path parameter \"{}\" was not found in request path", #name_string);
                    }
                },
            )
        } else {
            let (error_ty, map_err) = match &self.0.error {
                Some(error) => {
                    let construct = error.construct(&format!(
                        "invalid value for path parameter \"{name_string}\""
                    ));
                    (error.ty(), quote! { .map_err(|_| #construct) })
                }
                None => (
                    quote! { &'__cx <#inner_ty as ::core::str::FromStr>::Err },
                    quote! {},
                ),
            };
            (
                quote! {
                    ::core::result::Result<&'__cx #inner_ty, #error_ty>
                },
                quote! {
                    fn path_param(
                        cx: &::topcoat::context::Cx,
                        _: ::topcoat::router::PathParamSealed,
                    ) -> Self::Output<'_> {
                        #[::topcoat::context::memoize]
                        fn parse(cx: &::topcoat::context::Cx) -> ::core::result::Result<#ident #ty_generics, <#inner_ty as ::core::str::FromStr>::Err> {
                            for (key, value) in ::topcoat::router::raw_path_params(cx) {
                                if key == #name_string {
                                    return ::core::str::FromStr::from_str(value).map(#ident);
                                }
                            }
                            panic!("path parameter \"{}\" was not found in request path", #name_string);
                        }
                        parse(cx).map(|value| &value.0)#map_err
                    }
                },
            )
        };

        quote! {
            #item

            impl #impl_generics ::topcoat::router::PathParam for #ident #ty_generics #where_clause {
                type Output<'__cx> = #output_ty;

                #path_param_fn
            }

            ::topcoat::router::segment!(kind = Param, rename = #name_string);
        }
        .to_tokens(tokens);
    }
}
