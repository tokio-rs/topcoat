use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    LitStr,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::{
    template::TemplateExpr,
    view::{ExprKind, ViewWriter, WriteView},
};

/// The value part of an [`Attribute`](super::Attribute). Either a string
/// literal (`"foo"`) or a parenthesized Rust expression (`(expr)`) that is
/// evaluated at render time.
pub enum AttributeValue {
    Expr(Box<TemplateExpr>),
    LitStr(LitStr),
}

impl WriteView for AttributeValue {
    fn write(&self, writer: &mut ViewWriter) {
        match self {
            Self::Expr(inner) => {
                writer.write_expr(ExprKind::AttributeValue, inner.expr.to_token_stream());
            }
            Self::LitStr(inner) => writer.write_attribute_value(&inner.value()),
        }
    }
}

impl Parse for AttributeValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(Paren) {
            Ok(Self::Expr(input.parse()?))
        } else if lookahead.peek(LitStr) {
            Ok(Self::LitStr(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for AttributeValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(inner) => inner.to_tokens(tokens),
            Self::LitStr(inner) => inner.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for AttributeValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Expr(inner) => inner.pretty_print(printer),
            Self::LitStr(inner) => inner.pretty_print(printer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> AttributeValue {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<AttributeValue>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_literal_value() {
        let value = parse(r#""hello""#);
        let AttributeValue::LitStr(lit) = value else {
            panic!("expected literal value");
        };
        assert_eq!(lit.value(), "hello");
    }

    #[test]
    fn parses_expression_value() {
        assert!(matches!(parse("(value)"), AttributeValue::Expr(_)));
        assert!(matches!(parse("(a + b)"), AttributeValue::Expr(_)));
    }

    #[test]
    fn rejects_bare_identifier() {
        // Values must be quoted strings or parenthesized expressions.
        assert!(parse_err("value").contains("expected"));
    }
}
