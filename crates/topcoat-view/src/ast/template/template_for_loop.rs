use quote::quote;
use syn::{
    Expr, ExprBreak, ExprContinue, Pat, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

use crate::ast::{
    attributes::{AttributeWriter, WriteAttribute},
    template::TemplateBlock,
    view::{ViewWriter, WriteView},
};

/// A `for pat in expr { ... }` loop in view-body position. The body is
/// rendered once per iteration.
pub struct TemplateForLoop<T> {
    pub for_token: Token![for],
    pub pat: Box<Pat>,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: TemplateBlock<T>,
}

impl<T: WriteView> WriteView for TemplateForLoop<T> {
    fn write(&self, writer: &mut ViewWriter) {
        writer.for_loop(&self.pat, &self.expr, |writer| {
            self.body.write(writer);
        });
    }
}

impl<T: WriteAttribute> WriteAttribute for TemplateForLoop<T> {
    fn write(&self, writer: &mut AttributeWriter) {
        writer.for_loop(&self.pat, &self.expr, |writer| {
            self.body.write(writer);
        });
    }
}

impl<T: Parse> Parse for TemplateForLoop<T> {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            for_token: input.parse()?,
            pat: Box::new(input.call(Pat::parse_single)?),
            in_token: input.parse()?,
            expr: Box::new(input.call(Expr::parse_without_eager_brace)?),
            body: input.parse()?,
        })
    }
}

impl<T: Parse> ParseOption for TemplateForLoop<T> {
    fn peek(input: ParseStream) -> bool {
        // `for=` is an attribute named `for`, not the start of a loop.
        input.peek(Token![for]) && !input.peek2(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl<T: topcoat_pretty::PrettyPrint> topcoat_pretty::PrettyPrint for TemplateForLoop<T> {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.for_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.pat.pretty_print(printer);
        " ".pretty_print(printer);
        self.in_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.expr.pretty_print(printer);
        " ".pretty_print(printer);
        self.body.pretty_print(printer);
    }
}

/// A `continue;` statement.
pub struct TemplateContinue {
    pub expr_continue: ExprContinue,
    pub semi_token: Token![;],
}

impl WriteView for TemplateContinue {
    fn write(&self, writer: &mut ViewWriter) {
        let expr_continue = &self.expr_continue;
        writer.statement(quote! { #expr_continue; });
    }
}

impl WriteAttribute for TemplateContinue {
    fn write(&self, writer: &mut AttributeWriter) {
        let expr_continue = &self.expr_continue;
        writer.statement(quote! { #expr_continue; });
    }
}

impl Parse for TemplateContinue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_continue: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for TemplateContinue {
    fn peek(input: ParseStream) -> bool {
        // `continue=` is an attribute named `continue`, not a statement.
        input.peek(Token![continue]) && !input.peek2(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for TemplateContinue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use quote::ToTokens;

        self.expr_continue
            .to_token_stream()
            .to_string()
            .pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

/// A `break;` statement.
pub struct TemplateBreak {
    pub expr_break: ExprBreak,
    pub semi_token: Token![;],
}

impl WriteView for TemplateBreak {
    fn write(&self, writer: &mut ViewWriter) {
        let expr_break = &self.expr_break;
        writer.statement(quote! { #expr_break; });
    }
}

impl WriteAttribute for TemplateBreak {
    fn write(&self, writer: &mut AttributeWriter) {
        let expr_break = &self.expr_break;
        writer.statement(quote! { #expr_break; });
    }
}

impl Parse for TemplateBreak {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            expr_break: input.parse()?,
            semi_token: input.parse()?,
        })
    }
}

impl ParseOption for TemplateBreak {
    fn peek(input: ParseStream) -> bool {
        // `break=` is an attribute named `break`, not a statement.
        input.peek(Token![break]) && !input.peek2(Token![=])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for TemplateBreak {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use quote::ToTokens;

        self.expr_break
            .to_token_stream()
            .to_string()
            .pretty_print(printer);
        self.semi_token.pretty_print(printer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::view::Nodes;
    use quote::ToTokens;

    fn parse(source: &str) -> TemplateForLoop<Nodes> {
        syn::parse_str(source).unwrap()
    }

    #[test]
    fn parses_simple_for_loop() {
        let loop_ = parse(r"for x in xs { (x) }");
        assert_eq!(loop_.pat.to_token_stream().to_string(), "x");
        assert_eq!(loop_.expr.to_token_stream().to_string(), "xs");
        assert_eq!(loop_.body.children.len(), 1);
    }

    #[test]
    fn parses_tuple_pattern() {
        let loop_ = parse(r"for (k, v) in pairs { (k) }");
        assert_eq!(loop_.pat.to_token_stream().to_string(), "(k , v)");
    }

    #[test]
    fn parses_empty_body() {
        let loop_ = parse(r"for x in xs {}");
        assert!(loop_.body.children.is_empty());
    }

    #[test]
    fn parses_method_call_iterable() {
        // `parse_without_eager_brace` lets the `{` start the body rather than
        // being eaten by the expression.
        let loop_ = parse(r"for x in items.iter() { (x) }");
        assert_eq!(loop_.expr.to_token_stream().to_string(), "items . iter ()");
    }

    #[test]
    fn parses_continue_statement() {
        let c: TemplateContinue = syn::parse_str("continue;").unwrap();
        assert_eq!(c.expr_continue.to_token_stream().to_string(), "continue");
    }

    #[test]
    fn parses_break_statement() {
        let b: TemplateBreak = syn::parse_str("break;").unwrap();
        assert_eq!(b.expr_break.to_token_stream().to_string(), "break");
    }

    #[test]
    fn continue_and_break_require_trailing_semicolon() {
        assert!(syn::parse_str::<TemplateContinue>("continue").is_err());
        assert!(syn::parse_str::<TemplateBreak>("break").is_err());
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
    fn for_equals_is_an_attribute_not_a_loop() {
        assert!(peeks(TemplateForLoop::<Nodes>::peek, "for x in xs {}"));
        assert!(!peeks(TemplateForLoop::<Nodes>::peek, r#"for="email""#));
    }

    #[test]
    fn continue_and_break_equals_are_attributes() {
        assert!(peeks(TemplateContinue::peek, "continue;"));
        assert!(!peeks(TemplateContinue::peek, r#"continue="x""#));
        assert!(peeks(TemplateBreak::peek, "break;"));
        assert!(!peeks(TemplateBreak::peek, r#"break="x""#));
    }
}
