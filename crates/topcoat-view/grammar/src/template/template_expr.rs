use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;

use crate::view::{ExprKind, ViewWriter, WriteView};

/// A parenthesized Rust expression embedded as a child node, e.g. `(5 + 6)`.
#[derive(Debug, PartialEq)]
pub struct TemplateExpr {
    pub paren: syn::token::Paren,
    pub expr: syn::Expr,
}

impl WriteView for TemplateExpr {
    fn write(&self, writer: &mut ViewWriter) {
        let expr = &self.expr;
        writer.write_expr(ExprKind::Node, expr.to_token_stream());
    }
}

impl Parse for TemplateExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren: parenthesized!(content in input),
            expr: content.parse()?,
        })
    }
}

impl ParseOption for TemplateExpr {
    fn peek(input: ParseStream) -> bool {
        input.peek(syn::token::Paren)
    }
}

impl ToTokens for TemplateExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.expr.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TemplateExpr {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        "(".pretty_print(printer);
        self.expr.pretty_print(printer);
        ")".pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> TemplateExpr {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_plain_identifier() {
        let expr = parse("(value)");
        assert_eq!(expr.expr.to_token_stream().to_string(), "value");
    }

    #[test]
    fn parses_complex_expression() {
        let expr = parse("(a + b * c)");
        assert_eq!(expr.expr.to_token_stream().to_string(), "a + b * c");
    }

    #[test]
    fn parses_method_call() {
        let expr = parse("(user.name.clone())");
        assert_eq!(
            expr.expr.to_token_stream().to_string(),
            "user . name . clone ()",
        );
    }

    #[test]
    fn requires_parentheses() {
        assert!(syn::parse_str::<TemplateExpr>("value").is_err());
    }
}
