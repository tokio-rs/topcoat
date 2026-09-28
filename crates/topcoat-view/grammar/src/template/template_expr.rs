use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
};
use topcoat_core_grammar::ParseOption;

use crate::view::hir::{ExprKind, LowerView, ViewBuilder};

/// A parenthesized Rust expression embedded as a child node, e.g. `(5 + 6)`.
#[derive(Debug)]
pub struct TemplateExpr {
    pub paren: syn::token::Paren,
    pub expr: TokenStream,
}

impl PartialEq for TemplateExpr {
    fn eq(&self, other: &Self) -> bool {
        self.paren == other.paren
            && syn::Expr::Verbatim(self.expr.clone()) == syn::Expr::Verbatim(other.expr.clone())
    }
}

impl LowerView for TemplateExpr {
    fn lower(&self, builder: &mut ViewBuilder) {
        let expr = &self.expr;
        builder.expr(ExprKind::Node, expr.to_token_stream());
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
        syn::parse2::<syn::Expr>(self.expr.clone())
            .unwrap_or_else(|_| syn::Expr::Verbatim(self.expr.clone()))
            .pretty_print(printer);
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

    #[test]
    fn forwards_incomplete_expressions() {
        for source in [
            "value.",
            "String::",
            "call(,)",
            "{ let broken = ; value. }",
            "|value: String| value.",
        ] {
            let expr = parse(&format!("({source})"));
            let original: TokenStream = source.parse().unwrap();
            assert_eq!(expr.to_token_stream().to_string(), original.to_string());
        }
    }

    #[test]
    fn incomplete_expressions_expand_in_view_and_emit() {
        for source in [
            "<p>(value.)</p>",
            "<input value=(value.)>",
            "<input (value.)=\"text\">",
            "<input (value.)>",
            "<input :value=(value.)>",
            "<(value.)></(value.)>",
        ] {
            let view = syn::parse_str::<crate::view::View>(source).unwrap();
            let emit = syn::parse_str::<crate::live::Emit>(source).unwrap();
            for expanded in [view.to_token_stream(), emit.to_token_stream()] {
                assert!(expanded.to_string().contains("value ."), "{expanded}");
            }
        }
    }

    #[cfg(feature = "pretty")]
    #[test]
    fn formats_expression_tokens() {
        use topcoat_core_grammar::pretty::{Registry, pretty_print_str};

        let mut registry = Registry::new();
        registry.register_macro::<crate::view::View>("view");
        registry.register_macro::<crate::live::Emit>("emit");
        for name in ["view", "emit"] {
            for (input, expected) in [
                ("value . len ()", "value.len()"),
                ("value.", "value."),
                ("value /* keep */ .", "value /* keep */ ."),
            ] {
                let source = format!("{name}! {{ <p>({input})</p> }}");
                let formatted = pretty_print_str(&registry, &source).unwrap();
                assert!(formatted.contains(&format!("({expected})")), "{formatted}");
                assert_eq!(pretty_print_str(&registry, &formatted).unwrap(), formatted);
            }
        }
    }
}
