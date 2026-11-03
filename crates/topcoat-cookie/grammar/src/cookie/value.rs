use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Ident, Lit, LitStr,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};
use topcoat_core_grammar::common::ParseOption;

use crate::cookie::HeaderName;

/// The cookie name: written as in the header (`__Host-session`), as a string
/// literal, or as a parenthesized expression.
pub enum CookieName {
    Header(HeaderName),
    Lit(LitStr),
    Expr(ParenExpr),
}

impl CookieName {
    /// The name as a Rust expression.
    #[must_use]
    pub fn expr(&self) -> TokenStream {
        match self {
            Self::Header(name) => LitStr::new(&name.value(), name.span()).to_token_stream(),
            Self::Lit(lit) => lit.to_token_stream(),
            Self::Expr(expr) => expr.tokens.clone(),
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
    Expr(ParenExpr),
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
            Self::Expr(expr) => Ok(expr.tokens.clone()),
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

/// A Rust expression in parentheses, such as `(token)`. The tokens inside are
/// kept as written.
pub struct ParenExpr {
    pub paren_token: Paren,
    pub tokens: TokenStream,
}

impl Parse for ParenExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren_token: parenthesized!(content in input),
            tokens: content.parse()?,
        })
    }
}

/// Formats the expression inside the parentheses. Tokens that are not a complete
/// expression are copied as written.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::common::pretty::PrettyPrint for ParenExpr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::common::pretty::Printer<'_>) {
        use topcoat_core_grammar::common::pretty::{BreakMode, Delim};

        let expr = syn::parse2::<syn::Expr>(self.tokens.clone())
            .unwrap_or_else(|_| syn::Expr::Verbatim(self.tokens.clone()));
        self.paren_token
            .pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                expr.pretty_print(printer);
            });
    }
}
