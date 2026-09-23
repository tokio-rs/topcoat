use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
};

use crate::pretty::{BreakMode, Delim, PrettyPrint, Printer, Unspaced};

/// A wrapper type that parses and pretty-prints content with any of the three delimiter types.
///
/// - **Parentheses `()`**: No extra spacing around content
/// - **Braces `{}`**: Adds spaces around content (e.g., `{ foo }`), except when the body is empty,
///   which prints as `{}`
/// - **Brackets `[]`**: No extra spacing around content
pub enum Macro<T> {
    Parenthesized {
        paren: syn::token::Paren,
        inner: T,
    },
    Braced {
        brace: syn::token::Brace,
        empty: bool,
        inner: T,
    },
    Bracketed {
        bracket: syn::token::Bracket,
        inner: T,
    },
}

impl<T> Parse for Macro<T>
where
    T: Parse,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        let content;
        if lookahead.peek(syn::token::Paren) {
            Ok(Self::Parenthesized {
                paren: parenthesized!(content in input),
                inner: content.parse()?,
            })
        } else if lookahead.peek(syn::token::Brace) {
            let brace = braced!(content in input);
            // Collapse to `{}` only when nothing but whitespace sits between the
            // braces. Reading the source text (rather than scanning trivia) keeps
            // comments, which are not tokens, from being treated as empty.
            let empty = brace.span.join().source_text().is_some_and(|text| {
                text.strip_prefix('{')
                    .and_then(|text| text.strip_suffix('}'))
                    .is_some_and(|inner| inner.trim().is_empty())
            });
            Ok(Self::Braced {
                brace,
                empty,
                inner: content.parse()?,
            })
        } else if lookahead.peek(syn::token::Bracket) {
            Ok(Self::Bracketed {
                bracket: bracketed!(content in input),
                inner: content.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

impl<T> PrettyPrint for Macro<T>
where
    T: PrettyPrint,
{
    fn pretty_print(&self, printer: &mut Printer<'_>) {
        match self {
            Self::Parenthesized { paren, inner } => {
                paren.pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                    inner.pretty_print(printer);
                });
            }
            Self::Braced {
                brace,
                empty,
                inner,
            } => {
                if *empty {
                    Unspaced(brace).pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                        inner.pretty_print(printer);
                    });
                } else {
                    brace.pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                        inner.pretty_print(printer);
                    });
                }
            }
            Self::Bracketed { bracket, inner } => {
                bracket.pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                    inner.pretty_print(printer);
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::{
        Token,
        parse::{Parse, ParseStream},
    };

    use crate::pretty::{PrettyPrint, Printer, registry::Registry};

    /// A macro body used only in tests. It accepts an optional identifier, so
    /// an empty invocation such as `test! {}` still parses, optionally behind a
    /// `name:` prefix, which is what makes a body invalid Rust.
    struct Body {
        name: Option<syn::Ident>,
        value: Option<syn::Ident>,
    }

    impl Parse for Body {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let name = if input.peek(syn::Ident) && input.peek2(Token![:]) {
                let name = input.parse()?;
                input.parse::<Token![:]>()?;
                Some(name)
            } else {
                None
            };

            let value = if input.is_empty() {
                None
            } else {
                Some(input.parse()?)
            };

            Ok(Self { name, value })
        }
    }

    impl PrettyPrint for Body {
        fn pretty_print(&self, printer: &mut Printer<'_>) {
            if let Some(name) = &self.name {
                name.pretty_print(printer);
                ":".pretty_print(printer);
                " ".pretty_print(printer);
            }
            if let Some(value) = &self.value {
                value.pretty_print(printer);
            }
        }
    }

    fn registry() -> Registry {
        Registry::one::<Body>("test")
    }

    #[test]
    fn test_parenthesized_short() {
        let source = "test!(foo);";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test!(foo);");
    }

    #[test]
    fn test_parenthesized_long() {
        let source = "test!(name: this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed);";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            r"test!(
    name: this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed
);"
        );
    }

    #[test]
    fn parenthesized_rust_body_is_left_to_rustfmt() {
        // The body is a valid Rust expression, so `rustfmt` lays the invocation
        // out itself and this formatter leaves it untouched, however long it is.
        let source = "test!(this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed);";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), source);
    }

    #[test]
    fn test_braced_short() {
        let source = "test! { foo }";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! { foo }");
    }

    #[test]
    fn test_braced_empty() {
        let source = "test! {}";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! {}");
    }

    #[test]
    fn test_braced_empty_collapses_whitespace() {
        let source = "test! {   }";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! {}");
    }

    #[test]
    fn test_braced_empty_keeps_comment() {
        let source = "test! { /* keep me */ }";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! { /* keep me */ }");
    }

    #[test]
    fn braced_body_indents_to_macro_source_line() {
        // The macro is a call argument sitting at column 8. Its body must indent
        // relative to that column (to 12) and its closing brace back to it (8),
        // rather than following the shallow AST nesting depth of the call.
        let source = "\
fn f() {
    wrapper(
        first,
        test! { this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_printed },
    );
}
";
        let result = crate::pretty::pretty_print_str(&registry(), source).unwrap();
        assert!(
            result.contains(
                "        test! {\n            this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_printed\n        },"
            ),
            "unexpected output:\n{result}"
        );
    }

    #[test]
    fn test_braced_long() {
        let source = "test! { this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed }";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            r"test! {
    this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed
}"
        );
    }

    #[test]
    fn test_bracketed_short() {
        let source = "test![foo];";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test![foo];");
    }

    #[test]
    fn test_bracketed_long() {
        let source = "test![name: this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed];";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            r"test![
    name: this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed
];"
        );
    }

    #[test]
    fn bracketed_rust_body_is_left_to_rustfmt() {
        let source = "test![this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed];";
        let result = crate::pretty::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), source);
    }
}
