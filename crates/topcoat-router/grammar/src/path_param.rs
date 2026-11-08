use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Ident, Token, Type, Visibility,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::common::{
    ParseOption,
    paths::{topcoat_context, topcoat_context_macro, topcoat_router},
};

use super::common::ErrorResponse;

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
    pub(crate) fn is_catch_all(&self) -> bool {
        self.star_token.is_some()
    }

    fn param_type(&self) -> Option<&Type> {
        self.param_type.as_ref().map(|param_type| &param_type.ty)
    }

    fn error(&self) -> Option<&ErrorResponse> {
        self.error.as_ref().map(|error| &error.response)
    }

    /// The name as it appears in the URL, without a raw identifier prefix.
    pub(crate) fn name_string(&self) -> String {
        self.name.unraw().to_string()
    }

    /// Emits the function that reads the parameter from the matched route.
    fn emit_reader(&self, tokens: &mut TokenStream) {
        let visibility = &self.visibility;
        let ident = &self.name;
        let (output, body) = match (self.is_catch_all(), self.param_type()) {
            (false, Some(ty)) => (
                self.parsed_output(&quote! { &'__cx #ty }, ty),
                self.parse_segment(ty),
            ),
            (true, Some(ty)) => (
                self.parsed_output(&quote! { &'__cx [#ty] }, ty),
                self.parse_segments(ty),
            ),
            (false, None) => (quote! { &'__cx str }, self.read_segment()),
            (true, None) => (
                quote! { #topcoat_router::CatchAllSegments<'__cx> },
                self.read_segments(),
            ),
        };

        // A parameter can be declared only for a route to match it, so an
        // unread one does not warn.
        quote! {
            #[allow(dead_code)]
            #[track_caller]
            #visibility fn #ident<'__cx>(cx: &'__cx #topcoat_context::Cx) -> #output {
                #body
            }
        }
        .to_tokens(tokens);
    }

    /// Emits the marker type named after the parameter, through which `href!`
    /// fills the parameter with a value.
    ///
    /// The marker is an enum without variants, so it lives only in the type
    /// namespace and leaves the parameter's name free for the reader. `href!`
    /// calls its `__href` function, so `post_id: 5` reaches the marker as
    /// `post_id::__href(5)`.
    fn emit_marker(&self, tokens: &mut TokenStream) {
        let visibility = &self.visibility;
        let ident = &self.name;
        let name = self.name_string();
        let bound = self.value_bound();
        let push_segments = self.push_segments();

        quote! {
            #[doc(hidden)]
            #[allow(dead_code, non_camel_case_types)]
            #visibility enum #ident {}

            impl<V: #bound> #topcoat_router::HrefParam<V> for #ident {
                const NAME: &'static str = #name;

                fn segments(value: &V, segments: &mut #topcoat_router::HrefSegments<'_>) {
                    #push_segments
                }
            }

            impl #ident {
                #[doc(hidden)]
                #[allow(dead_code)]
                #visibility fn __href<V: #bound>(value: V) -> #topcoat_router::HrefArg<#ident, V> {
                    <#ident as #topcoat_router::HrefParam<V>>::arg(value)
                }
            }
        }
        .to_tokens(tokens);
    }

    /// The bound on the values `href!` accepts for the parameter.
    pub(crate) fn value_bound(&self) -> TokenStream {
        match (self.is_catch_all(), self.param_type()) {
            (false, Some(ty)) => quote! { ::core::borrow::Borrow<#ty> },
            (true, Some(ty)) => quote! { ::core::convert::AsRef<[#ty]> },
            (false, None) => quote! { ::core::convert::AsRef<str> },
            (true, None) => quote! { #topcoat_router::SegmentList },
        }
    }

    /// Pushes the segments of `value` onto `segments`: one for a regular
    /// parameter, one per element for a catch-all.
    fn push_segments(&self) -> TokenStream {
        match (self.is_catch_all(), self.param_type()) {
            (false, Some(ty)) => {
                quote! { segments.push(::core::borrow::Borrow::<#ty>::borrow(value)); }
            }
            (true, Some(ty)) => quote! {
                for segment in ::core::convert::AsRef::<[#ty]>::as_ref(value) {
                    segments.push(segment);
                }
            },
            (false, None) => {
                quote! { segments.push(::core::convert::AsRef::<str>::as_ref(value)); }
            }
            (true, None) => quote! {
                #topcoat_router::SegmentList::for_each_segment(value, |segment| {
                    segments.push(segment);
                });
            },
        }
    }

    /// The result of a parsed parameter: the borrowed `value`, or the declared
    /// error response, or the borrowed parse error without one.
    fn parsed_output(&self, value: &TokenStream, ty: &Type) -> TokenStream {
        let error = self.error().map_or_else(
            || quote! { &'__cx <#ty as ::core::str::FromStr>::Err },
            ErrorResponse::ty,
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

    /// Parses the segment once per request and borrows the cached result.
    fn parse_segment(&self, ty: &Type) -> TokenStream {
        let segment = self.read_segment();
        let map_err = self.map_err();

        quote! {
            #[#topcoat_context_macro::memoize(as_ref)]
            fn parse(
                cx: &#topcoat_context::Cx,
            ) -> ::core::result::Result<#ty, <#ty as ::core::str::FromStr>::Err> {
                ::core::str::FromStr::from_str(#segment)
            }

            parse(cx)#map_err
        }
    }

    /// Parses every segment once per request and borrows the cached result. A
    /// failure keeps the index of the segment that failed.
    fn parse_segments(&self, ty: &Type) -> TokenStream {
        let segments = self.read_segments();
        let map_err = self.map_err();

        quote! {
            #[#topcoat_context_macro::memoize(as_ref)]
            fn parse(
                cx: &#topcoat_context::Cx,
            ) -> ::core::result::Result<
                ::std::vec::Vec<#ty>,
                (usize, <#ty as ::core::str::FromStr>::Err),
            > {
                #segments
                    .enumerate()
                    .map(|(index, segment)| {
                        ::core::str::FromStr::from_str(segment)
                            .map_err(|error| (index, error))
                    })
                    .collect::<::core::result::Result<::std::vec::Vec<#ty>, _>>()
            }

            parse(cx).map(|values| &values[..])#map_err
        }
    }

    fn map_err(&self) -> TokenStream {
        let name = self.name_string();

        match (self.error(), self.is_catch_all()) {
            (Some(error), false) => error.map_err(quote! {
                |_| #topcoat_router::error::bad_request(
                    ::std::format!("invalid value for path parameter \"{}\"", #name)
                )
            }),
            (Some(error), true) => error.map_err(quote! {
                |(index, _)| #topcoat_router::error::bad_request(
                    ::std::format!(
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
                error.else_token.span,
                "`else` cannot be used with an unparsed path parameter, which cannot fail",
            ));
        }

        Ok(param)
    }
}

impl ToTokens for PathParam {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.emit_reader(tokens);
        self.emit_marker(tokens);
    }
}

/// Prints the declaration on one line. A trailing comma is dropped.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for PathParam {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.visibility.pretty_print(printer);
        self.star_token.pretty_print(printer);
        self.name.pretty_print(printer);
        self.param_type.pretty_print(printer);
        if let Some(error) = &self.error {
            " ".pretty_print(printer);
            error.pretty_print(printer);
        }
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

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for PathParamType {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.ty.pretty_print(printer);
    }
}

/// The `else` clause of a `path_param!` declaration, choosing the error
/// response returned when the parameter fails to parse.
pub struct PathParamError {
    pub else_token: Token![else],
    pub response: ErrorResponse,
}

impl Parse for PathParamError {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            else_token: input.parse()?,
            response: input.parse()?,
        })
    }
}

impl ParseOption for PathParamError {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![else])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for PathParamError {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        self.else_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.response.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use quote::ToTokens;

    use super::*;
    use crate::common::ErrorKind;

    fn parse(source: &str) -> PathParam {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_a_typed_catch_all_with_visibility_and_error() {
        let param = parse("pub *post_ids: u64 else bad_request,");

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
    }

    #[test]
    fn parses_an_else_clause_with_arguments() {
        let param = parse(r#"post_id: u64 else bad_request("invalid id")"#);

        let response = param.error().unwrap();
        assert!(matches!(response.kind, ErrorKind::BadRequest(_)));
        assert_eq!(response.args.as_ref().unwrap().args.len(), 1);
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
    }

    #[test]
    fn rejects_error_on_an_unparsed_parameter() {
        let error = syn::parse_str::<PathParam>("slug else not_found")
            .err()
            .unwrap();

        assert_eq!(
            error.to_string(),
            "`else` cannot be used with an unparsed path parameter, which cannot fail"
        );
    }

    #[test]
    fn removes_raw_identifier_prefix_from_names() {
        let param = parse("r#type: u32");

        assert_eq!(param.name_string(), "type");
    }
}
