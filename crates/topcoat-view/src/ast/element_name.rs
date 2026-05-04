use std::fmt::Display;

use proc_macro2::Span;
use quote::quote;
use syn::{
    Expr, Ident, LitStr,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::Paren,
};

use crate::output::ViewWriter;

/// The name appearing in an [`Element`](super::Element)'s tag. May be a plain
/// identifier (`div`), a string literal (`"my-tag"`), or a parenthesized Rust
/// expression that resolves to the tag name at runtime.
#[derive(PartialEq, Eq)]
pub enum ElementName {
    Ident(Ident),
    LitStr(LitStr),
    Expr { paren: Paren, expr: Box<Expr> },
}

impl ElementName {
    /// The tag name as a string when it is statically known. Returns `None` for
    /// expression-valued names, which can only be resolved at runtime.
    pub fn string_name(&self) -> Option<String> {
        match self {
            Self::Ident(inner) => Some(inner.to_string()),
            Self::LitStr(inner) => Some(inner.value()),
            Self::Expr { .. } => None,
        }
    }

    /// The source span covering the name.
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(inner) => inner.span(),
            Self::LitStr(inner) => inner.span(),
            Self::Expr { paren, .. } => paren.span.span(),
        }
    }

    pub(crate) fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Ident(inner) => writer.write_str_unescaped(&inner.to_string()),
            Self::LitStr(inner) => writer.write_str_unescaped(&inner.value()),
            Self::Expr { expr, .. } => writer.write_expr(quote! { #expr }),
        }
    }

    /// Returns `true` if this name is one of the HTML void elements (`br`,
    /// `img`, `input`, …) — those that take no closing tag and no children.
    /// Only matches identifier names; string and expression names always
    /// return `false`.
    pub fn is_void_element(&self) -> bool {
        const VOID_ELEMENTS: &[&str] = &[
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
            "track", "wbr",
        ];

        match self {
            Self::Ident(inner) => {
                let name = inner.to_string();
                VOID_ELEMENTS.iter().any(|v| *v == name)
            }
            _ => false,
        }
    }

    /// Returns the underlying expression if this name was written as
    /// `(expr)`, otherwise `None`.
    pub fn expr(&self) -> Option<&Expr> {
        match self {
            Self::Expr { expr, .. } => Some(expr),
            _ => None,
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
            Ok(Self::Ident(input.call(Ident::parse_any)?))
        } else if lookahead.peek(LitStr) {
            Ok(Self::LitStr(input.parse()?))
        } else if lookahead.peek(Paren) {
            let content;
            Ok(Self::Expr {
                paren: parenthesized!(content in input),
                expr: content.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for ElementName {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::Ident(inner) => inner.pretty_print(printer),
            Self::LitStr(inner) => inner.pretty_print(printer),
            Self::Expr { paren, expr } => {
                use topcoat_pretty::{BreakMode, Delim};

                paren.pretty_print(printer, Some(BreakMode::Inconsistent), |printer| {
                    expr.pretty_print(printer);
                });
            }
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
