use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Ident, Lit, LitStr,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::common::{ParseOption, lenient};

use crate::cookie::HeaderName;

/// The cookie name: written as in the header (`__Host-session`), as a string
/// literal, or as a parenthesized expression.
pub enum CookieName {
    Header(HeaderName),
    Lit(LitStr),
    Expr(lenient::ExprParen),
}

impl CookieName {
    /// The name as a Rust expression.
    #[must_use]
    pub fn expr(&self) -> TokenStream {
        match self {
            Self::Header(name) => LitStr::new(&name.value(), name.span()).to_token_stream(),
            Self::Lit(lit) => lit.to_token_stream(),
            Self::Expr(paren) => paren.expr.to_token_stream(),
        }
    }
}

impl Parse for CookieName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            Ok(Self::Lit(input.parse()?))
        } else if lookahead.peek(Paren) {
            Ok(Self::Expr(input.parse()?))
        } else if lookahead.peek(Ident::peek_any) {
            Ok(Self::Header(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for CookieName {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        match self {
            Self::Header(name) => name.pretty_print(printer),
            Self::Lit(lit) => lit.token().pretty_print(printer),
            Self::Expr(expr) => expr.pretty_print(printer),
        }
    }
}

/// The value of a cookie or an attribute: a literal, a parenthesized expression,
/// or a bare name such as the `Lax` in `SameSite = Lax`.
pub enum CookieValue {
    Lit(Lit),
    Expr(lenient::ExprParen),
    Name(Ident),
}

impl CookieValue {
    /// The value as a Rust expression.
    ///
    /// # Errors
    ///
    /// Returns an error for a bare name, which only some attributes accept.
    pub fn expr(&self) -> syn::Result<TokenStream> {
        match self {
            Self::Lit(lit) => Ok(lit.to_token_stream()),
            Self::Expr(paren) => Ok(paren.expr.to_token_stream()),
            Self::Name(name) => Err(syn::Error::new(
                name.span(),
                "expected a literal or a parenthesized expression",
            )),
        }
    }
}

impl Parse for CookieValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Lit) {
            Ok(Self::Lit(input.parse()?))
        } else if lookahead.peek(Paren) {
            Ok(Self::Expr(input.parse()?))
        } else if lookahead.peek(Ident::peek_any) {
            Ok(Self::Name(input.call(Ident::parse_any)?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ParseOption for CookieValue {
    fn peek(input: ParseStream) -> bool {
        input.peek(Lit) || input.peek(Paren) || input.peek(Ident::peek_any)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for CookieValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        match self {
            Self::Lit(lit) => lit.pretty_print(printer),
            Self::Expr(expr) => expr.pretty_print(printer),
            Self::Name(name) => name.pretty_print(printer),
        }
    }
}
