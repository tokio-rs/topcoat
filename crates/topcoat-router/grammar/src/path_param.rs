use heck::ToPascalCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{
    Ident, Token, Type, Visibility,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::{
    ParseOption,
    paths::{topcoat_context, topcoat_context_macro, topcoat_router, topcoat_router_macro},
};

use super::common::ErrorAttr;

/// The input to `path_param!`.
pub struct PathParam {
    pub visibility: Visibility,
    pub star_token: Option<Token![*]>,
    pub name: Ident,
    pub param_type: Option<PathParamType>,
    pub error: Option<PathParamError>,
    pub trailing_comma: Option<Token![,]>,
}

impl PathParam {
    fn is_catch_all(&self) -> bool {
        self.star_token.is_some()
    }

    fn param_type(&self) -> Option<&Type> {
        self.param_type.as_ref().map(|param_type| &param_type.ty)
    }

    fn error(&self) -> Option<&ErrorAttr> {
        self.error.as_ref().map(|error| &error.error)
    }

    fn type_ident(&self) -> Ident {
        format_ident!(
            "{}",
            self.name.unraw().to_string().to_pascal_case(),
            span = self.name.span()
        )
    }

    fn name_string(&self) -> String {
        self.name.unraw().to_string()
    }

    fn item(&self) -> TokenStream {
        let visibility = &self.visibility;
        let ident = self.type_ident();

        match (self.is_catch_all(), self.param_type()) {
            (false, Some(param_type)) => {
                quote! { #visibility struct #ident(#visibility #param_type); }
            }
            (true, Some(param_type)) => {
                quote! {
                    #visibility struct #ident(
                        #visibility ::std::vec::Vec<#param_type>
                    );
                }
            }
            (false, None) => quote! {
                #visibility struct #ident<
                    T: ::core::convert::AsRef<str> = ::std::string::String,
                >(#visibility T);
            },
            (true, None) => quote! {
                #visibility struct #ident<
                    T: ::core::iter::IntoIterator<Item: ::core::convert::AsRef<str>>
                        = ::std::vec::Vec<::std::string::String>,
                >(#[allow(dead_code)] #visibility T);
            },
        }
    }

    fn path_param_impl(&self) -> TokenStream {
        let ident = self.type_ident();
        let (impl_generics, output, body) = match (self.is_catch_all(), self.param_type()) {
            (false, Some(param_type)) => (
                quote! {},
                self.parsed_output(param_type, &quote! { &'__cx #param_type }),
                self.parse_segment(param_type),
            ),
            (true, Some(param_type)) => (
                quote! {},
                self.parsed_output(param_type, &quote! { &'__cx [#param_type] }),
                self.parse_segments(param_type),
            ),
            (false, None) => (
                quote! { <T: ::core::convert::AsRef<str>> },
                quote! { &'__cx str },
                self.read_segment(),
            ),
            (true, None) => (
                quote! {
                    <T: ::core::iter::IntoIterator<Item: ::core::convert::AsRef<str>>>
                },
                quote! { #topcoat_router::CatchAllSegments<'__cx> },
                self.read_segments(),
            ),
        };
        let type_generics = self.param_type().is_none().then(|| quote! { <T> });

        quote! {
            impl #impl_generics #topcoat_router::PathParam for #ident #type_generics {
                type Output<'__cx> = #output;

                #[track_caller]
                fn path_param(
                    cx: &#topcoat_context::Cx,
                    _: #topcoat_router::PathParamSealed,
                ) -> Self::Output<'_> {
                    #body
                }
            }
        }
    }

    fn parsed_output(&self, param_type: &Type, value: &TokenStream) -> TokenStream {
        let error = self.error().map_or_else(
            || quote! { &'__cx <#param_type as ::core::str::FromStr>::Err },
            ErrorAttr::ty,
        );

        quote! { ::core::result::Result<#value, #error> }
    }

    fn read_segment(&self) -> TokenStream {
        let name = self.name_string();
        quote! { #topcoat_router::path_param_segment(cx, #name) }
    }

    fn read_segments(&self) -> TokenStream {
        let name = self.name_string();
        quote! { #topcoat_router::path_param_segments(cx, #name) }
    }

    fn parse_segment(&self, param_type: &Type) -> TokenStream {
        let ident = self.type_ident();
        let segment = self.read_segment();
        let map_err = self.map_err();

        quote! {
            #[#topcoat_context_macro::memoize(as_ref)]
            fn parse(
                cx: &#topcoat_context::Cx,
            ) -> ::core::result::Result<
                #ident,
                <#param_type as ::core::str::FromStr>::Err,
            > {
                ::core::str::FromStr::from_str(#segment).map(#ident)
            }

            parse(cx).map(|value| &value.0)#map_err
        }
    }

    fn parse_segments(&self, param_type: &Type) -> TokenStream {
        let ident = self.type_ident();
        let segments = self.read_segments();
        let map_err = self.map_err();

        quote! {
            #[#topcoat_context_macro::memoize(as_ref)]
            fn parse(
                cx: &#topcoat_context::Cx,
            ) -> ::core::result::Result<
                #ident,
                (usize, <#param_type as ::core::str::FromStr>::Err),
            > {
                #segments
                    .enumerate()
                    .map(|(index, segment)| {
                        ::core::str::FromStr::from_str(segment)
                            .map_err(|error| (index, error))
                    })
                    .collect::<::core::result::Result<::std::vec::Vec<#param_type>, _>>()
                    .map(#ident)
            }

            parse(cx).map(|value| &value.0[..])#map_err
        }
    }

    fn map_err(&self) -> TokenStream {
        let name = self.name_string();

        match (self.error(), self.is_catch_all()) {
            (Some(error), false) => error.map_err(quote! {
                |_| #topcoat_router::error::bad_request(
                    format!("invalid value for path parameter \"{}\"", #name)
                )
            }),
            (Some(error), true) => error.map_err(quote! {
                |(index, _)| #topcoat_router::error::bad_request(
                    format!(
                        "invalid value for path parameter \"{}\" at segment {}",
                        #name,
                        index,
                    )
                )
            }),
            (None, false) => quote! {},
            (None, true) => quote! { .map_err(|error| &error.1) },
        }
    }

    fn href_param_impl(&self) -> TokenStream {
        let ident = self.type_ident();
        let name = self.name_string();
        let (impl_generics, where_clause, value_type, value_expr) =
            match (self.is_catch_all(), self.param_type()) {
                (false, Some(param_type)) => (
                    quote! {},
                    quote! { where #param_type: ::core::fmt::Display },
                    quote! { #param_type },
                    quote! { &self.0 },
                ),
                (true, Some(param_type)) => (
                    quote! {},
                    quote! { where #param_type: ::core::fmt::Display },
                    quote! { Self },
                    quote! { self },
                ),
                (false, None) => (
                    quote! { <T: ::core::convert::AsRef<str>> },
                    quote! {},
                    quote! { str },
                    quote! { ::core::convert::AsRef::<str>::as_ref(&self.0) },
                ),
                (true, None) => (
                    quote! {
                        <T: ::core::iter::IntoIterator<Item: ::core::convert::AsRef<str>>>
                    },
                    quote! {
                        where for<'__iter> &'__iter T:
                            ::core::iter::IntoIterator<Item: ::core::convert::AsRef<str>>
                    },
                    quote! { Self },
                    quote! { self },
                ),
            };
        let type_generics = self.param_type().is_none().then(|| quote! { <T> });
        let display_impl = self.href_display_impl();

        quote! {
            #display_impl

            impl #impl_generics #topcoat_router::HrefParam
                for #ident #type_generics #where_clause
            {
                type Value = #value_type;

                fn name(&self) -> &str {
                    #name
                }

                fn value(&self) -> &Self::Value {
                    #value_expr
                }
            }
        }
    }

    // A catch-all fills several segments at once, so the parameter itself is
    // the href value and displays as its segments joined by `/`.
    fn href_display_impl(&self) -> Option<TokenStream> {
        if !self.is_catch_all() {
            return None;
        }

        let ident = self.type_ident();
        let Some(param_type) = self.param_type() else {
            return Some(quote! {
                impl<T: ::core::iter::IntoIterator<Item: ::core::convert::AsRef<str>>>
                    ::core::fmt::Display for #ident<T>
                where
                    for<'__iter> &'__iter T:
                        ::core::iter::IntoIterator<Item: ::core::convert::AsRef<str>>,
                {
                    fn fmt(
                        &self,
                        f: &mut ::core::fmt::Formatter<'_>,
                    ) -> ::core::fmt::Result {
                        for (index, segment) in (&self.0).into_iter().enumerate() {
                            if index > 0 {
                                f.write_str("/")?;
                            }
                            f.write_str(::core::convert::AsRef::<str>::as_ref(&segment))?;
                        }
                        ::core::result::Result::Ok(())
                    }
                }
            });
        };

        Some(quote! {
            impl ::core::fmt::Display for #ident
            where
                #param_type: ::core::fmt::Display,
            {
                fn fmt(
                    &self,
                    f: &mut ::core::fmt::Formatter<'_>,
                ) -> ::core::fmt::Result {
                    for (index, value) in self.0.iter().enumerate() {
                        if index > 0 {
                            f.write_str("/")?;
                        }
                        ::core::fmt::Display::fmt(value, f)?;
                    }
                    ::core::result::Result::Ok(())
                }
            }
        })
    }

    fn segment(&self) -> TokenStream {
        let name = self.name_string();
        let kind = if self.is_catch_all() {
            quote! { CatchAll }
        } else {
            quote! { Param }
        };

        quote! { #topcoat_router_macro::segment!(kind = #kind, rename = #name); }
    }
}

impl Parse for PathParam {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let param = Self {
            visibility: input.parse()?,
            star_token: input.parse()?,
            name: input.parse()?,
            param_type: input.call(PathParamType::parse_option)?,
            error: input.call(PathParamError::parse_option)?,
            trailing_comma: input.peek(Token![,]).then(|| input.parse()).transpose()?,
        };

        if let Some(error) = &param.error
            && param.param_type().is_none()
        {
            return Err(syn::Error::new(
                error.error.span(),
                "`error` cannot be used with an unparsed path parameter, which cannot fail",
            ));
        }

        Ok(param)
    }
}

impl ToTokens for PathParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let item = self.item();
        let path_param_impl = self.path_param_impl();
        let href_param_impl = self.href_param_impl();
        let segment = self.segment();

        quote! {
            #item
            #path_param_impl
            #href_param_impl
            #segment
        }
        .to_tokens(tokens);
    }
}

/// A parsed segment type in a `path_param!` declaration.
pub struct PathParamType {
    pub colon_token: Token![:],
    pub ty: Type,
}

impl Parse for PathParamType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}

impl ParseOption for PathParamType {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![:])
    }
}

/// An error mapping in a `path_param!` declaration.
pub struct PathParamError {
    pub comma_token: Token![,],
    pub error: ErrorAttr,
}

impl Parse for PathParamError {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            comma_token: input.parse()?,
            error: input.parse()?,
        })
    }
}

impl ParseOption for PathParamError {
    fn peek(input: ParseStream) -> bool {
        let input = input.fork();
        if input.parse::<Token![,]>().is_err() {
            return false;
        }
        ErrorAttr::peek(&input)
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;

    fn parse(source: &str) -> PathParam {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_a_typed_catch_all_with_visibility_and_error() {
        let param = parse("pub *post_ids: u64, error = bad_request,");

        assert!(matches!(param.visibility, Visibility::Public(_)));
        assert!(param.is_catch_all());
        assert_eq!(param.name, "post_ids");
        assert_eq!(
            param
                .param_type
                .as_ref()
                .unwrap()
                .ty
                .to_token_stream()
                .to_string(),
            "u64"
        );
        assert!(param.error.is_some());
        assert!(param.trailing_comma.is_some());
        assert_eq!(param.type_ident(), "PostIds");
    }

    #[test]
    fn parses_an_unparsed_single_segment() {
        let param = parse("slug");

        assert!(matches!(param.visibility, Visibility::Inherited));
        assert!(!param.is_catch_all());
        assert_eq!(param.name, "slug");
        assert!(param.param_type.is_none());
        assert!(param.error.is_none());
        assert!(param.trailing_comma.is_none());
        assert_eq!(param.type_ident(), "Slug");
    }

    #[test]
    fn rejects_error_on_an_unparsed_parameter() {
        let error = syn::parse_str::<PathParam>("slug, error = not_found")
            .err()
            .unwrap();

        assert_eq!(
            error.to_string(),
            "`error` cannot be used with an unparsed path parameter, which cannot fail"
        );
    }

    #[test]
    fn removes_raw_identifier_prefix_from_names() {
        let param = parse("r#type: u32");

        assert_eq!(param.name_string(), "type");
        assert_eq!(param.type_ident(), "Type");
    }
}
