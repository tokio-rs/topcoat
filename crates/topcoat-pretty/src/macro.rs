use syn::{
    braced, bracketed, parenthesized,
    parse::{Parse, ParseStream},
};

use crate::{BreakMode, Delim, PrettyPrint, Printer};

/// A wrapper type that parses and pretty-prints content with any of the three delimiter types.
///
/// This enum is used by the Topcoat formatter (`cargo topcoat fmt`) to scan macro invocations
/// in Rust source files and reformat their contents while preserving the original delimiter
/// choice. Different delimiter types receive different formatting treatment:
///
/// - **Parentheses `()`**: No extra spacing around content
/// - **Braces `{}`**: Adds spaces around content (e.g., `{ foo }`)
/// - **Brackets `[]`**: No extra spacing around content
///
/// # Usage
///
/// The formatter uses this type with [`pretty_print_macro_str`] to reformat macro contents:
///
/// ```rust
/// // Parse and reformat a macro invocation based on its delimiter
/// let result = pretty_print_macro_str::<Macro<topcoat_view::ast::view::View>>(
///     "{ <html><body>"format"</body></html> }",
///     initial_space,
///     initial_indent,
/// )?;
/// ```
///
/// When the formatter encounters a macro like `view! { ... }` or `view!( ... )`,
/// it extracts the delimiter span and uses this type to parse and reformat the contents
/// according to the delimiter type used.
///
/// [`pretty_print_macro_str`]: topcoat_pretty::pretty_print_macro_str
pub enum Macro<T> {
    Parenthesized {
        paren: syn::token::Paren,
        inner: T,
    },
    Braced {
        brace: syn::token::Brace,
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
            Ok(Self::Braced {
                brace: braced!(content in input),
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
            Self::Braced { brace, inner } => {
                brace.pretty_print(printer, Some(BreakMode::Consistent), |printer| {
                    inner.pretty_print(printer);
                });
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
    use crate::registry::Registry;

    fn registry() -> Registry {
        Registry::one::<syn::Ident>("test")
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
