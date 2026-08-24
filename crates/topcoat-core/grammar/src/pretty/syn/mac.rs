use syn::{parse::Parser, punctuated::Punctuated, spanned::Spanned};

use super::common;
use crate::pretty::{BreakMode, Delim, MacroSnippet, PrettyPrint, Printer, TextMode};

impl PrettyPrint for syn::Macro {
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        // A registered Topcoat macro is formatted by its own pretty-printer.
        if let Some(snippet) = MacroSnippet::from_macro(self, printer.current_indent())
            && let Some(Ok(formatted)) = printer.registry().pretty_print_macro(&snippet)
        {
            self.path.pretty_print(printer);
            self.bang_token.pretty_print(printer);
            if matches!(self.delimiter, syn::MacroDelimiter::Brace(_)) {
                " ".pretty_print(printer);
            }
            printer.move_cursor(snippet.span().start());
            printer.scan_text(formatted.into(), TextMode::Always);
            printer.move_cursor(snippet.span().end());
            printer.skip_trivia();
            return;
        }

        // A body that is a comma-separated list of expressions (`format!`,
        // `assert!`, ...) lays out like a call's argument list.
        if !matches!(self.delimiter, syn::MacroDelimiter::Brace(_))
            && let Ok(args) = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated
                .parse2(self.tokens.clone())
        {
            self.path.pretty_print(printer);
            self.bang_token.pretty_print(printer);
            let print_args = |printer: &mut Printer<'_>| args.pretty_print(printer);
            match &self.delimiter {
                syn::MacroDelimiter::Paren(paren) => {
                    paren.pretty_print(printer, Some(BreakMode::Consistent), print_args);
                }
                syn::MacroDelimiter::Bracket(bracket) => {
                    bracket.pretty_print(printer, Some(BreakMode::Consistent), print_args);
                }
                syn::MacroDelimiter::Brace(_) => unreachable!("brace bodies are copied verbatim"),
            }
            return;
        }

        // Everything else is copied through unchanged; reprinting an arbitrary
        // token stream could separate tokens a grammar requires to be adjacent.
        self.path.pretty_print(printer);
        self.bang_token.pretty_print(printer);
        if matches!(self.delimiter, syn::MacroDelimiter::Brace(_)) {
            " ".pretty_print(printer);
        }
        let span = self.delimiter.span().span();
        common::verbatim_span(printer, span, || match &self.delimiter {
            syn::MacroDelimiter::Paren(_) => format!("({})", self.tokens),
            syn::MacroDelimiter::Bracket(_) => format!("[{}]", self.tokens),
            syn::MacroDelimiter::Brace(_) => format!("{{ {} }}", self.tokens),
        });
    }
}

#[cfg(test)]
mod tests {
    use syn::{
        Token,
        parse::{Parse, ParseStream},
    };

    use super::super::common::tests::{format, format_with_registry};
    use crate::pretty::{PrettyPrint, Printer, Registry};

    fn expr(source: &str) -> String {
        format::<syn::Expr>(source)
    }

    #[test]
    fn expression_body_lays_out_like_a_call() {
        assert_eq!(expr("format!(\"{}\", value)"), "format!(\"{}\", value)");
        assert_eq!(
            expr("format ! ( \"{}\" , value )"),
            "format!(\"{}\", value)",
        );
    }

    #[test]
    fn long_expression_body_breaks() {
        assert_eq!(
            expr(
                "format!(\"{} and {}\", first_extremely_long_argument_value, second_extremely_long_argument_value)"
            ),
            "format!(\n    \"{} and {}\",\n    first_extremely_long_argument_value,\n    second_extremely_long_argument_value,\n)",
        );
    }

    #[test]
    fn bracketed_macro() {
        assert_eq!(expr("vec![1, 2, 3]"), "vec![1, 2, 3]");
    }

    #[test]
    fn or_patterns_parse_as_expressions() {
        assert_eq!(
            expr("matches!(status, Status::Idle | Status::Done)"),
            "matches!(status, Status::Idle | Status::Done)",
        );
    }

    #[test]
    fn unparseable_body_is_copied_verbatim() {
        assert_eq!(expr("vec![0u8; 16]"), "vec![0u8; 16]");
        assert_eq!(expr("weird!(aria-label #foo)"), "weird!(aria-label #foo)",);
    }

    #[test]
    fn comment_in_expression_body_is_kept() {
        assert_eq!(
            expr("assert!(/* invariant */ ready)"),
            "assert!(/* invariant */ ready)",
        );
    }

    /// A macro body for registry tests: `name: value`, which is not valid Rust,
    /// so only the registered pretty-printer can format it.
    struct Body {
        name: syn::Ident,
        value: syn::Ident,
    }

    impl Parse for Body {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let name = input.parse()?;
            input.parse::<Token![:]>()?;
            Ok(Self {
                name,
                value: input.parse()?,
            })
        }
    }

    impl PrettyPrint for Body {
        fn pretty_print(&self, printer: &mut Printer<'_>) {
            self.name.pretty_print(printer);
            ":".pretty_print(printer);
            " ".pretty_print(printer);
            self.value.pretty_print(printer);
        }
    }

    #[test]
    fn registered_macro_uses_its_own_printer() {
        let registry = Registry::one::<Body>("test");
        assert_eq!(
            format_with_registry::<syn::Expr>(&registry, "wrap(test!(name :  value))"),
            "wrap(test!(name: value))",
        );
    }

    #[test]
    fn registered_macro_is_formatted_even_when_rustfmt_would_lay_it_out() {
        // At file level a parenthesized body of valid Rust is left to
        // `rustfmt`, but inside another macro body `rustfmt` never reaches it.
        let registry = Registry::one::<Body>("test");
        assert_eq!(
            format_with_registry::<syn::Expr>(
                &registry,
                "wrap(test!(name: this_is_a_very_long_identifier_name_that_should_definitely_break_across_lines))"
            ),
            "wrap(\n    test!(\n        name: this_is_a_very_long_identifier_name_that_should_definitely_break_across_lines\n    ),\n)",
        );
    }
}
