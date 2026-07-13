use std::fmt::Display;

use proc_macro2::Span;
use quote::ToTokens;
use syn::{
    Expr, Ident, LitStr,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Paren,
};

use crate::{
    template::TemplateExpr,
    view::{ExprKind, HtmlIdent, ViewWriter, WriteView},
};

/// The name appearing in an [`Element`](super::Element)'s tag. May be an HTML
/// identifier (`div`, `data-foo`, `xmlns:xlink`), a string literal
/// (`"my-tag"`), or a parenthesized Rust expression that resolves to the tag
/// name at runtime.
#[derive(Debug, PartialEq)]
pub enum ElementName {
    Ident(HtmlIdent),
    LitStr(LitStr),
    Expr(Box<TemplateExpr>),
}

impl ElementName {
    /// The tag name as a string when it is statically known. Returns `None` for
    /// expression-valued names, which can only be resolved at runtime.
    #[must_use]
    pub fn string_name(&self) -> Option<String> {
        match self {
            Self::Ident(inner) => Some(inner.to_string()),
            Self::LitStr(inner) => Some(inner.value()),
            Self::Expr { .. } => None,
        }
    }

    /// The source span covering the name.
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(inner) => inner.span(),
            Self::LitStr(inner) => inner.span(),
            Self::Expr(inner) => inner.paren.span.span(),
        }
    }

    /// Returns `true` if this name is one of the HTML void elements (`br`,
    /// `img`, `input`, ...): those that take no closing tag and no children.
    /// Only matches identifier names; string and expression names always
    /// return `false`.
    #[must_use]
    pub fn is_void_element(&self) -> bool {
        const VOID_ELEMENTS: &[&str] = &[
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
            "track", "wbr",
        ];

        match self {
            Self::Ident(inner) if inner.rest.is_empty() => {
                let name = inner.first.to_string();
                VOID_ELEMENTS.iter().any(|v| *v == name)
            }
            _ => false,
        }
    }

    /// Returns the underlying expression if this name was written as
    /// `(expr)`, otherwise `None`.
    #[must_use]
    pub fn expr(&self) -> Option<&Expr> {
        match self {
            Self::Expr(inner) => Some(&inner.expr),
            _ => None,
        }
    }
}

impl WriteView for ElementName {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Ident(inner) => writer.write_str_unescaped(&inner.to_string()),
            Self::LitStr(inner) => writer.write_str_unescaped(&inner.value()),
            Self::Expr(inner) => {
                writer.write_expr(ExprKind::ElementName, inner.expr.to_token_stream());
            }
        }
    }
}

impl Display for ElementName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ident(inner) => inner.fmt(f),
            Self::LitStr(inner) => inner.value().fmt(f),
            Self::Expr { .. } => f.write_str("<expr>"),
        }
    }
}

impl Parse for ElementName {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            Ok(Self::Ident(HtmlIdent::parse_dash_only(input)?))
        } else if lookahead.peek(LitStr) {
            Ok(Self::LitStr(input.parse()?))
        } else if lookahead.peek(Paren) {
            Ok(Self::Expr(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for ElementName {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Ident(inner) => inner.pretty_print(printer),
            Self::LitStr(inner) => inner.pretty_print(printer),
            Self::Expr(inner) => inner.pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> ElementName {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn ident_name_returns_string_name() {
        let name = parse("div");
        assert_eq!(name.string_name().as_deref(), Some("div"));
        assert!(name.expr().is_none());
    }

    #[test]
    fn html_ident_name_allows_dashes() {
        assert_eq!(
            parse("my-component").string_name().as_deref(),
            Some("my-component"),
        );
        assert_eq!(
            parse("data-foo-bar").string_name().as_deref(),
            Some("data-foo-bar"),
        );
    }

    #[test]
    fn html_ident_name_stops_at_colon_or_dot() {
        // `:` and `.` are reserved for attribute syntax (`:value`,
        // `class.active`) and must not be consumed as part of an element name.
        use syn::Token;
        use syn::parse::Parser;

        let parser = |input: syn::parse::ParseStream| -> syn::Result<ElementName> {
            let name = input.parse::<ElementName>()?;
            let _: Token![:] = input.parse()?;
            let _: Ident = input.parse()?;
            Ok(name)
        };
        let name = parser.parse_str("xmlns:xlink").unwrap();
        assert_eq!(name.string_name().as_deref(), Some("xmlns"));

        let parser = |input: syn::parse::ParseStream| -> syn::Result<ElementName> {
            let name = input.parse::<ElementName>()?;
            let _: Token![.] = input.parse()?;
            let _: Ident = input.parse()?;
            Ok(name)
        };
        let name = parser.parse_str("class.active").unwrap();
        assert_eq!(name.string_name().as_deref(), Some("class"));
    }

    #[test]
    fn html_ident_void_check_ignores_multi_segment_names() {
        // A multi-segment HTML identifier whose first segment matches a void
        // element name should not be treated as void.
        assert!(!parse("br-custom").is_void_element());
    }

    #[test]
    fn lit_str_name_returns_string_name() {
        let name = parse(r#""my-tag""#);
        assert_eq!(name.string_name().as_deref(), Some("my-tag"));
        assert!(name.expr().is_none());
    }

    #[test]
    fn expression_name_has_no_string_name() {
        let name = parse("(tag)");
        assert!(name.string_name().is_none());
        assert!(name.expr().is_some());
    }

    #[test]
    fn is_void_element_only_matches_known_void_idents() {
        for tag in [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
            "track", "wbr",
        ] {
            assert!(parse(tag).is_void_element(), "{tag} should be void");
        }
        assert!(!parse("div").is_void_element());
    }

    #[test]
    fn is_void_element_ignores_string_and_expr_names() {
        // Even spelling a void tag as a string literal or expression must not
        // count as void.
        assert!(!parse(r#""br""#).is_void_element());
        assert!(!parse("(br)").is_void_element());
    }
}
