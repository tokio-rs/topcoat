use proc_macro2::extra::DelimSpan;
use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
};

use crate::{BreakMode, Delim, PrettyPrint, Printer};

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

/// Wraps a [`Delim`] to suppress the spacing it would otherwise add around its
/// body, so an empty braced macro prints as `{}` rather than `{  }` while still
/// reusing the delimiter's cursor and trivia handling.
struct Unspaced<'a, D>(&'a D);

impl<D> Delim for Unspaced<'_, D>
where
    D: Delim,
{
    fn space(&self) -> bool {
        false
    }

    fn open_text(&self) -> &'static str {
        self.0.open_text()
    }

    fn close_text(&self) -> &'static str {
        self.0.close_text()
    }

    fn span(&self) -> DelimSpan {
        self.0.span()
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
    use syn::parse::{Parse, ParseStream};

    use crate::{PrettyPrint, Printer, registry::Registry};

    /// A macro body used only in tests that accepts an optional identifier, so
    /// an empty invocation such as `test! {}` still parses.
    struct Body(Option<syn::Ident>);

    impl Parse for Body {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            if input.is_empty() {
                Ok(Self(None))
            } else {
                Ok(Self(Some(input.parse()?)))
            }
        }
    }

    impl PrettyPrint for Body {
        fn pretty_print(&self, printer: &mut Printer<'_>) {
            if let Some(ident) = &self.0 {
                ident.pretty_print(printer);
            }
        }
    }

    fn registry() -> Registry {
        Registry::one::<Body>("test")
    }

    #[test]
    fn test_parenthesized_short() {
        let source = "test!(foo);";
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test!(foo);");
    }

    #[test]
    fn test_parenthesized_long() {
        let source = "test!(this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed);";
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            r"test!(
    this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed
);"
        );
    }

    #[test]
    fn test_braced_short() {
        let source = "test! { foo }";
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! { foo }");
    }

    #[test]
    fn test_braced_empty() {
        let source = "test! {}";
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! {}");
    }

    #[test]
    fn test_braced_empty_collapses_whitespace() {
        let source = "test! {   }";
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! {}");
    }

    #[test]
    fn test_braced_empty_keeps_comment() {
        let source = "test! { /* keep me */ }";
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test! { /* keep me */ }");
    }

    #[test]
    fn test_braced_long() {
        let source = "test! { this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed }";
        let result = crate::pretty_print_str(&registry(), source);
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
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test![foo];");
    }

    #[test]
    fn test_bracketed_long() {
        let source = "test![this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed];";
        let result = crate::pretty_print_str(&registry(), source);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            r"test![
    this_is_a_very_long_identifier_name_that_should_definitely_break_across_multiple_lines_when_pretty_printed
];"
        );
    }
}
