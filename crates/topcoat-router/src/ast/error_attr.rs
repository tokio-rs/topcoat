use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Expr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Paren,
};

mod kw {
    syn::custom_keyword!(error);
    syn::custom_keyword!(bad_request);
    syn::custom_keyword!(forbidden);
    syn::custom_keyword!(not_found);
    syn::custom_keyword!(redirect);
    syn::custom_keyword!(redirect_permanent);
    syn::custom_keyword!(unauthorized);
}

/// The `error = ...` macro argument shared by the request parameter macros.
///
/// It names one of the router's error constructors, optionally with call
/// arguments (`error = not_found`, `error = bad_request("no such post")`),
/// and stands for the user-facing error response returned when the parameter
/// fails to parse.
pub struct ErrorAttr {
    kind: ErrorKind,
    args: Vec<Expr>,
}

impl ErrorAttr {
    /// The span of the constructor name, for attaching validation errors.
    pub fn span(&self) -> Span {
        self.kind.keyword().span()
    }

    /// The router error type the constructor produces.
    pub fn ty(&self) -> TokenStream {
        match self.kind {
            ErrorKind::BadRequest(_) => quote! { ::topcoat::router::BadRequestError },
            ErrorKind::Forbidden(_) => quote! { ::topcoat::router::ForbiddenError },
            ErrorKind::NotFound(_) => quote! { ::topcoat::router::NotFoundError },
            ErrorKind::Redirect(_) | ErrorKind::RedirectPermanent(_) => {
                quote! { ::topcoat::router::RedirectError }
            }
            ErrorKind::Unauthorized(_) => quote! { ::topcoat::router::UnauthorizedError },
        }
    }

    /// A call expression constructing the error.
    ///
    /// `default_description` fills `bad_request`'s description when the
    /// attribute does not provide one. Argument arity and types are left to
    /// the compiler, which checks the generated constructor call.
    pub fn construct(&self, default_description: &str) -> TokenStream {
        let name = self.kind.keyword();
        let args = if matches!(self.kind, ErrorKind::BadRequest(_)) && self.args.is_empty() {
            quote! { #default_description }
        } else {
            let args = &self.args;
            quote! { #(#args),* }
        };
        quote! { ::topcoat::router::#name(#args) }
    }
}

impl Parse for ErrorAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input.parse::<kw::error>()?;
        input.parse::<Token![=]>()?;
        Ok(Self {
            kind: input.parse()?,
            args: if input.peek(Paren) {
                let content;
                syn::parenthesized!(content in input);
                Punctuated::<Expr, Token![,]>::parse_terminated(&content)?
                    .into_iter()
                    .collect()
            } else {
                Vec::new()
            },
        })
    }
}

enum ErrorKind {
    BadRequest(kw::bad_request),
    Forbidden(kw::forbidden),
    NotFound(kw::not_found),
    Redirect(kw::redirect),
    RedirectPermanent(kw::redirect_permanent),
    Unauthorized(kw::unauthorized),
}

impl ErrorKind {
    /// The keyword naming the router constructor the attribute calls.
    fn keyword(&self) -> &dyn ToTokens {
        match self {
            Self::BadRequest(kw) => kw,
            Self::Forbidden(kw) => kw,
            Self::NotFound(kw) => kw,
            Self::Redirect(kw) => kw,
            Self::RedirectPermanent(kw) => kw,
            Self::Unauthorized(kw) => kw,
        }
    }
}

impl Parse for ErrorKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::bad_request) {
            Ok(Self::BadRequest(input.parse()?))
        } else if lookahead.peek(kw::forbidden) {
            Ok(Self::Forbidden(input.parse()?))
        } else if lookahead.peek(kw::not_found) {
            Ok(Self::NotFound(input.parse()?))
        } else if lookahead.peek(kw::redirect) {
            Ok(Self::Redirect(input.parse()?))
        } else if lookahead.peek(kw::redirect_permanent) {
            Ok(Self::RedirectPermanent(input.parse()?))
        } else if lookahead.peek(kw::unauthorized) {
            Ok(Self::Unauthorized(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}
