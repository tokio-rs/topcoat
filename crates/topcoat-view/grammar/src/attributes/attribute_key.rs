use std::fmt::{self, Display};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Ident,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    token::Paren,
};

use topcoat_core_grammar::ParseOption;

use crate::{
    template::TemplateExpr,
    view::{ExprKind, HtmlIdent, ViewWriter, WriteView},
};

/// The name part of a single `name=value` attribute on an
/// [`Element`](crate::view::Element) or [`Component`](crate::view::Component). Either an
/// HTML identifier (`data-foo`, `aria-label`) or a parenthesized Rust
/// expression that resolves to the attribute name at runtime.
pub enum AttributeKey {
    Ident(HtmlIdent),
    Expr(Box<TemplateExpr>),
}

impl AttributeKey {
    /// Returns `true` if the attribute key is [`Ident`].
    ///
    /// [`Ident`]: AttributeKey::Ident
    #[must_use]
    pub fn is_ident(&self) -> bool {
        matches!(self, Self::Ident(..))
    }

    #[must_use]
    pub fn as_ident(&self) -> Option<&HtmlIdent> {
        if let Self::Ident(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the attribute key is [`Expr`].
    ///
    /// [`Expr`]: AttributeKey::Expr
    #[must_use]
    pub fn is_expr(&self) -> bool {
        matches!(self, Self::Expr(..))
    }

    #[must_use]
    pub fn as_expr(&self) -> Option<&TemplateExpr> {
        if let Self::Expr(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl WriteView for AttributeKey {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Ident(inner) => writer.write_str_unescaped(&inner.to_string()),
            Self::Expr(inner) => {
                writer.write_expr(ExprKind::AttributeKey, inner.expr.to_token_stream());
            }
        }
    }
}

impl Parse for AttributeKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Paren) {
            Ok(Self::Expr(input.parse()?))
        } else if lookahead.peek(Ident::peek_any) {
            Ok(Self::Ident(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ParseOption for AttributeKey {
    fn peek(input: ParseStream) -> bool {
        TemplateExpr::peek(input) || HtmlIdent::peek(input)
    }
}

impl ToTokens for AttributeKey {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(inner) => inner.to_string().to_tokens(tokens),
            Self::Expr(inner) => inner.expr.to_tokens(tokens),
        }
    }
}

impl Display for AttributeKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(inner) => Display::fmt(inner, f),
            Self::Expr(_) => f.write_str("<expr>"),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for AttributeKey {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Ident(inner) => inner.pretty_print(printer),
            Self::Expr(inner) => inner.pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> AttributeKey {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_plain_ident() {
        let key = parse("class");
        assert!(matches!(key, AttributeKey::Ident(_)));
        assert_eq!(key.to_string(), "class");
    }

    #[test]
    fn parses_html_ident_with_separators() {
        assert_eq!(parse("data-foo").to_string(), "data-foo");
        assert_eq!(parse("xmlns:xlink").to_string(), "xmlns:xlink");
        assert_eq!(parse("class.active").to_string(), "class.active");
    }

    #[test]
    fn parses_rust_keyword_as_key() {
        // `type` and `for` are valid HTML attribute names.
        assert_eq!(parse("type").to_string(), "type");
        assert_eq!(parse("for").to_string(), "for");
    }

    #[test]
    fn parses_expression_key() {
        let key = parse("(name)");
        assert!(matches!(key, AttributeKey::Expr(_)));
        assert_eq!(key.to_string(), "<expr>");
    }
}
