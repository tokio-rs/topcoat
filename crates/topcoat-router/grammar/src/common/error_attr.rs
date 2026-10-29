use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use super::ErrorResponse;

mod kw {
    syn::custom_keyword!(error);
}

/// The `error = ...` option of an attribute macro, choosing the error
/// response returned when a request parameter fails to parse.
pub struct ErrorAttr {
    pub error_token: kw::error,
    pub eq_token: Token![=],
    pub response: ErrorResponse,
}

impl Parse for ErrorAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            error_token: input.parse()?,
            eq_token: input.parse()?,
            response: input.parse()?,
        })
    }
}
