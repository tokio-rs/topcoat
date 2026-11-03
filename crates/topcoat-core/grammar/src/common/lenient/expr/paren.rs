use std::fmt;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    token::Paren,
};

use crate::common::{ParseOption, lenient::Expr};

/// A Rust expression in parentheses, such as `(user.name)`.
///
/// The parentheses mark where the expression ends, so every token inside is
/// kept as written, even code that is not a valid expression yet. Empty
/// parentheses hold an empty [`Expr`], which expands to a compile error.
///
/// Like [`syn::ExprParen`], it expands to the expression in its parentheses.
/// Use the [`expr`](Self::expr) field for the expression alone.
#[derive(Clone)]
pub struct ExprParen {
    pub paren_token: Paren,
    pub expr: Expr,
}

impl Parse for ExprParen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            paren_token: parenthesized!(content in input),
            expr: Expr {
                span: content.span(),
                tokens: content.parse()?,
            },
        })
    }
}

impl ParseOption for ExprParen {
    fn peek(input: ParseStream) -> bool {
        input.peek(Paren)
    }
}

impl ToTokens for ExprParen {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.paren_token
            .surround(tokens, |tokens| self.expr.to_tokens(tokens));
    }
}

/// Compares the expressions, ignoring spans.
impl PartialEq for ExprParen {
    fn eq(&self, other: &Self) -> bool {
        self.expr == other.expr
    }
}

impl fmt::Debug for ExprParen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ExprParen")
            .field("expr", &self.expr)
            .finish_non_exhaustive()
    }
}

/// Formats the expression inside the parentheses, which hug it, so a block
/// stays `({ ... })`. Tokens that are not a complete expression are copied as
/// written.
#[cfg(feature = "pretty")]
impl crate::common::pretty::PrettyPrint for ExprParen {
    fn pretty_print(&self, printer: &mut crate::common::pretty::Printer<'_>) {
        printer.move_cursor(self.paren_token.span.open().start());
        "(".pretty_print(printer);
        printer.move_cursor(self.paren_token.span.open().end());
        self.expr.pretty_print(printer);
        printer.move_cursor(self.paren_token.span.close().start());
        ")".pretty_print(printer);
        printer.move_cursor(self.paren_token.span.close().end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> ExprParen {
        syn::parse_str(source).unwrap()
    }

    fn tokens(source: &str) -> String {
        source.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn keeps_every_token_inside_the_parentheses() {
        for source in [
            "a + b",
            "value.",
            "String::",
            "call(,)",
            "a b",
            "{ let broken = ; value. }",
            "|value: String| value.",
        ] {
            let paren = parse(&format!("({source})"));
            assert_eq!(paren.expr.tokens.to_string(), tokens(source), "{source}");
        }
    }

    #[test]
    fn requires_parentheses() {
        assert!(syn::parse_str::<ExprParen>("value").is_err());
    }

    #[test]
    fn expands_empty_parentheses_to_a_compile_error() {
        let expanded = parse("()").to_token_stream().to_string();
        assert!(expanded.contains("compile_error"), "{expanded}");
    }

    #[test]
    fn expands_to_the_expression_in_parentheses() {
        let paren = parse("(user.)");
        assert_eq!(paren.to_token_stream().to_string(), tokens("(user.)"));
        assert_eq!(paren.expr.to_token_stream().to_string(), tokens("user."));
    }
}
