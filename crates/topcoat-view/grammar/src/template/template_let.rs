use syn::{
    ExprLet, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core_grammar::ParseOption;

use crate::{
    attributes::{AttributeWriter, WriteAttribute},
    view::{ViewWriter, WriteView},
};

/// A `let pat = expr;` binding in view-body position. The binding is in scope
/// for all sibling nodes that follow it.
pub struct TemplateLet {
    pub expr_let: ExprLet,
    pub semi_token: Token![;],
}

impl WriteView for TemplateLet {
    fn write(&self, writer: &mut ViewWriter) {
        writer.let_binding(&self.expr_let.pat, &self.expr_let.expr);
    }
}

impl WriteAttribute for TemplateLet {
    fn write(&self, writer: &mut AttributeWriter) {
        writer.let_binding(&self.expr_let.pat, &self.expr_let.expr);
    }
}

impl Parse for TemplateLet {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_let: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for TemplateLet {
    fn peek(input: ParseStream) -> bool {
        // `let=` is an attribute named `let`, not the start of a binding.
        input.peek(Token![let]) && !input.peek2(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TemplateLet {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.expr_let.pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::ToTokens;

    fn parse(source: &str) -> TemplateLet {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_identifier_binding() {
        let let_ = parse("let x = 1;");
        assert_eq!(let_.expr_let.pat.to_token_stream().to_string(), "x");
        assert_eq!(let_.expr_let.expr.to_token_stream().to_string(), "1");
    }

    #[test]
    fn parses_destructuring_pattern() {
        let let_ = parse("let (a, b) = pair;");
        assert_eq!(let_.expr_let.pat.to_token_stream().to_string(), "(a , b)");
    }

    #[test]
    fn requires_trailing_semicolon() {
        assert!(syn::parse_str::<TemplateLet>("let x = 1").is_err());
    }

    /// Evaluates a `peek` against `source`, draining the remaining tokens so the
    /// surrounding `parse_str` doesn't error on the unconsumed input.
    fn peeks(peek: fn(ParseStream) -> bool, source: &str) -> bool {
        let parser = move |input: ParseStream| -> syn::Result<bool> {
            let peeked = peek(input);
            input.parse::<proc_macro2::TokenStream>()?;
            Ok(peeked)
        };
        syn::parse::Parser::parse_str(parser, source).unwrap()
    }

    #[test]
    fn let_equals_is_an_attribute_not_a_binding() {
        assert!(peeks(TemplateLet::peek, "let x = 1;"));
        assert!(!peeks(TemplateLet::peek, r#"let="x""#));
    }
}
