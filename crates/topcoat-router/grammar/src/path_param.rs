use proc_macro2::TokenStream;
use quote::{ToTokens};
use syn::{
    Ident, Token, Type, Visibility,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::common::{
    ParseOption,
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
        let visibility = &self.visibility;
        let 

        quote! {

        }.to_tokens(tokens);
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
        assert_eq!(param.type_ident(), "PostIds");
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
        assert_eq!(param.type_ident(), "Slug");
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
        assert_eq!(param.type_ident(), "Type");
    }
}
