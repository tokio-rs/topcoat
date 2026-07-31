use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Expr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Paren,
};
use topcoat_core_grammar::{ParseOption, paths::topcoat_router};

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
    pub error_token: kw::error,
    pub eq_token: Token![=],
    pub kind: ErrorKind,
    pub args: Option<ErrorArgs>,
}

impl ErrorAttr {
    /// The span of the constructor name, for attaching validation errors.
    #[must_use]
    pub fn span(&self) -> Span {
        self.kind.keyword().span()
    }

    /// The router error type the constructor produces.
    #[must_use]
    pub fn ty(&self) -> TokenStream {
        match self.kind {
            ErrorKind::BadRequest(_) => quote! { #topcoat_router::error::BadRequestError },
            ErrorKind::Forbidden(_) => quote! { #topcoat_router::error::ForbiddenError },
            ErrorKind::NotFound(_) => quote! { #topcoat_router::error::NotFoundError },
            ErrorKind::Redirect(_) | ErrorKind::RedirectPermanent(_) => {
                quote! { #topcoat_router::error::RedirectError }
            }
            ErrorKind::Unauthorized(_) => quote! { #topcoat_router::error::UnauthorizedError },
        }
    }

    /// The `.map_err(...)` adapter replacing a failed parse's error with the
    /// declared response.
    ///
    /// A bare `bad_request` carries no description to fill the constructor
    /// with, so the macro's `default_bad_request` handler (a closure from the
    /// original parse error to the response) is used instead. All other
    /// constructor calls are checked by the compiler.
    #[must_use]
    pub fn map_err(&self, default_bad_request: TokenStream) -> TokenStream {
        let args = self
            .args
            .as_ref()
            .map_or_else(Punctuated::new, |args| args.args.clone());
        let handler = if matches!(self.kind, ErrorKind::BadRequest(_)) && args.is_empty() {
            default_bad_request
        } else {
            let name = self.kind.keyword();
            quote! { |_| #topcoat_router::error::#name(#args) }
        };
        quote! { .map_err(#handler) }
    }
}

impl Parse for ErrorAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            error_token: input.parse()?,
            eq_token: input.parse()?,
            kind: input.parse()?,
            args: input.call(ErrorArgs::parse_option)?,
        })
    }
}

impl ParseOption for ErrorAttr {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::error)
    }
}

pub enum ErrorKind {
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

/// The parenthesized arguments passed to an error constructor.
pub struct ErrorArgs {
    pub paren_token: Paren,
    pub args: Punctuated<Expr, Token![,]>,
}

impl Parse for ErrorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren_token: syn::parenthesized!(content in input),
            args: Punctuated::parse_terminated(&content)?,
        })
    }
}

impl ParseOption for ErrorArgs {
    fn peek(input: ParseStream) -> bool {
        input.peek(Paren)
    }
}
