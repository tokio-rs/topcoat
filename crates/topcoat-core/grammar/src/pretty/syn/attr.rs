use syn::spanned::Spanned;

use super::common;
use crate::pretty::{MacroSnippet, PrettyPrint, Printer, TextMode};

impl PrettyPrint for syn::Attribute {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        // Comments still pending before the attribute must come out now: the
        // attribute's source text is copied verbatim and the trivia it covers
        // is dropped afterwards, which would silently swallow them.
        printer.scan_trivia(true, true);
        if self.meta.path().is_ident("doc") {
            // Doc comments are captured by the trivia lexer and reproduced like
            // regular comments, so the parsed attribute prints nothing.
            return;
        }

        self.pound_token.pretty_print(printer);
        if let syn::AttrStyle::Inner(not) = &self.style {
            not.pretty_print(printer);
        }
        // The arguments of a registered attribute are formatted by its own
        // pretty-printer.
        if let syn::Meta::List(list) = &self.meta
            && let Some(snippet) = MacroSnippet::from_attribute(list, printer.current_indent())
            && let Some(Ok(formatted)) = printer.registry().pretty_print_macro(&snippet)
        {
            common::token(printer, "[", self.bracket_token.span.open());
            list.path.pretty_print(printer);
            printer.move_cursor(snippet.span().start());
            printer.scan_text(formatted.into(), TextMode::Always);
            printer.move_cursor(snippet.span().end());
            printer.skip_trivia();
            common::token(printer, "]", self.bracket_token.span.close());
        } else if let Some(source_text) = self.bracket_token.span.span().source_text() {
            source_text.pretty_print(printer);
        }
        printer.move_cursor(self.bracket_token.span.close().end());
        printer.skip_trivia();
        printer.scan_same_line_trivia();
        printer.scan_break();
        " ".pretty_print(printer);
        printer.scan_trivia(true, true);
    }
}

#[cfg(test)]
mod tests {
    use super::super::common::tests::{format, format_with_registry};
    use crate::pretty::Registry;

    fn block(source: &str) -> String {
        format::<syn::Block>(source)
    }

    #[test]
    fn registered_attribute_uses_its_own_printer() {
        let mut registry = Registry::new();
        registry.register_attribute::<syn::Expr>("check");
        assert_eq!(
            format_with_registry::<syn::Block>(&registry, "{ #[check(a  +  b)] fn f() {} }"),
            "{\n    #[check(a + b)]\n    fn f() {}\n}",
        );
    }

    #[test]
    fn function_like_registration_does_not_apply_to_attributes() {
        let registry = Registry::one::<syn::Expr>("check");
        assert_eq!(
            format_with_registry::<syn::Block>(&registry, "{ #[check(a  +  b)] fn f() {} }"),
            "{\n    #[check(a  +  b)]\n    fn f() {}\n}",
        );
    }

    #[test]
    fn attribute_on_statement() {
        assert_eq!(
            block("{ #[allow(unused)] let x = 1; }"),
            "{\n    #[allow(unused)]\n    let x = 1;\n}",
        );
    }

    #[test]
    fn attribute_source_is_kept_verbatim() {
        assert_eq!(
            block("{ #[cfg(feature = \"extra\")] run(); }"),
            "{\n    #[cfg(feature = \"extra\")]\n    run();\n}",
        );
    }

    #[test]
    fn doc_comment_before_attribute() {
        assert_eq!(
            block("{\n    /// Documented.\n    #[allow(unused)]\n    fn helper() {}\n}"),
            "{\n    /// Documented.\n    #[allow(unused)]\n    fn helper() {}\n}",
        );
    }
}
