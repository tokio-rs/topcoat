use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Data, DeriveInput, Fields, Type,
    parse::{Parse, ParseStream},
};

pub struct PathParamAttr;

impl Parse for PathParamAttr {
    fn parse(_input: ParseStream) -> syn::Result<Self> {
        Ok(Self)
    }
}

pub struct PathParamItem {
    item: DeriveInput,
    inner_ty: Type,
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
    pub fn new(attr: PathParamAttr, item: PathParamItem) -> Self {
        Self(attr, item)
    }

    pub fn parse(attr: TokenStream, item: TokenStream) -> syn::Result<Self> {
        Ok(Self::new(syn::parse2(attr)?, syn::parse2(item)?))
    }
}

impl ToTokens for PathParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = &self.1.item;
        let ident = &item.ident;
        let inner_ty = &self.1.inner_ty;
        let name_string = ident.to_string().to_snake_case();
        let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

        fn is_str_ref(ty: &Type) -> bool {
            let Type::Reference(reference) = ty else {
                return false;
            };
            if reference.mutability.is_some() {
                return false;
            }
            let Type::Path(path) = &*reference.elem else {
                return false;
            };
            path.qself.is_none() && path.path.is_ident("str")
        }

        let of_fn = if is_str_ref(inner_ty) {
            quote! {
                fn of(cx: &::topcoat::context::Cx) -> &ident {
                    for (key, value) in ::topcoat::router::raw_path_params(cx) {
                        if key == #name_string {
                            return #ident(value);
                        }
                    }
                    panic!("path parameter \"{}\" was not found in request path", #name_string);
                }
            }
        } else {
            quote! {
                fn of<'__cx>(
                    cx: &'__cx ::topcoat::context::Cx,
                ) -> ::core::result::Result<&'__cx #ident, &'__cx <#inner_ty as ::core::str::FromStr>::Err> {
                    #[::topcoat::context::memoize]
                    fn parse(cx: &::topcoat::context::Cx) -> ::core::result::Result<#ident, <#inner_ty as ::core::str::FromStr>::Err> {
                        for (key, value) in ::topcoat::router::raw_path_params(cx) {
                            if key == #name_string {
                                return ::core::str::FromStr::from_str(value).map(#ident);
                            }
                        }
                        panic!("path parameter \"{}\" was not found in request path", #name_string);
                    }
                    parse(cx)
                }
            }
        };

        quote! {
            #item

            impl #impl_generics #ident #ty_generics #where_clause {
                #of_fn
            }

            impl ::core::ops::Deref for #ident {
                type Target = #inner_ty;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            ::topcoat::router::segment!(kind = Param, rename = #name_string);
        }
        .to_tokens(tokens);
    }
}
