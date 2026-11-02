use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::{ParseOption, common::UnparsedExpr};

use crate::fluent::{DocComments, FluentIdent, FluentIdentToken};

/// Arguments supplied after the key in `t!`.
///
/// Argument names must be unique.
pub struct TranslateArguments {
    /// The optional comma before the argument list.
    pub comma_token: Option<Token![,]>,
    /// The supplied message arguments.
    pub arguments: Punctuated<TranslateArgument, Token![,]>,
}

impl Parse for TranslateArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let comma_token: Option<Token![,]> = input.parse()?;
        let arguments = match comma_token {
            Some(_) => Punctuated::<TranslateArgument, _>::parse_terminated(input)?,
            None => Punctuated::new(),
        };
        let names: Vec<_> = arguments
            .iter()
            .map(|argument| argument.name.value())
            .collect();
        for (index, argument) in arguments.iter().enumerate() {
            let name = &names[index];
            if names[..index].contains(name) {
                return Err(syn::Error::new_spanned(
                    &argument.name,
                    format!("the argument `{name}` is passed more than once"),
                ));
            }
        }
        Ok(Self {
            comma_token,
            arguments,
        })
    }
}

/// Separates the arguments with a comma and a break. The last argument gets a comma
/// only when the list is split over several lines. A comma without arguments is
/// dropped.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TranslateArguments {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use topcoat_core_grammar::pretty::TextMode;

        let Some(comma_token) = &self.comma_token else {
            return;
        };
        if self.arguments.is_empty() {
            printer.move_cursor(comma_token.span.end());
            return;
        }

        comma_token.pretty_print(printer);
        for (index, pair) in self.arguments.pairs().enumerate() {
            printer.scan_same_line_trivia();
            printer.scan_break();
            " ".pretty_print(printer);
            printer.scan_trivia(true, true);
            pair.value().pretty_print(printer);
            if pair.punct().is_some() {
                printer.scan_no_break_trivia();
            }
            match pair.punct() {
                Some(punct) if index < self.arguments.len() - 1 => punct.pretty_print(printer),
                _ => {
                    printer.scan_text(",".into(), TextMode::Break);
                    printer.advance_cursor(",");
                }
            }
        }
    }
}

/// A named message argument, written as `name: value` or just `name` to use a Rust
/// variable with that name.
///
/// Names identify Fluent variables, so `user_name` supplies `$user_name`. Use a string
/// literal for names that are not Rust identifiers, such as `"user-name": name`. The
/// shorthand requires a Rust identifier. A raw identifier such as `r#type` supplies
/// `$type`.
pub struct TranslateArgument {
    /// Documentation attached to this argument.
    pub docs: DocComments,
    /// The Fluent variable to fill.
    pub name: FluentIdent,
    /// The explicit value, absent for shorthand arguments.
    pub value: Option<TranslateArgumentValue>,
}

impl TranslateArgument {
    /// Returns the supplied value expression, or the variable name for shorthand
    /// arguments.
    ///
    /// An incomplete value produces a compile error at `:` while preserving the
    /// argument's place in the expansion.
    #[must_use]
    pub fn value(&self) -> TokenStream {
        match &self.value {
            None => self.name.to_token_stream(),
            Some(value) if value.expr.is_empty() => syn::Error::new(
                value.colon_token.span,
                format!("expected a value for `{}` after `:`", self.name.value()),
            )
            .to_compile_error(),
            Some(value) => value.expr.tokens.clone(),
        }
    }
}

impl Parse for TranslateArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let argument = Self {
            docs: input.parse()?,
            name: input.parse()?,
            value: input.call(TranslateArgumentValue::parse_option)?,
        };
        let is_rust_ident = matches!(
            &argument.name,
            FluentIdent::Tokens(tokens) if matches!(tokens.as_slice(), [FluentIdentToken::Ident(_)])
        );
        if argument.value.is_none() && !is_rust_ident {
            return Err(syn::Error::new_spanned(
                &argument.name,
                format!(
                    "the shorthand only accepts a Rust identifier; write `{}: value`",
                    argument.name.to_token_stream()
                ),
            ));
        }
        Ok(argument)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TranslateArgument {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.docs.pretty_print(printer);
        self.name.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

/// An explicit value in a [`TranslateArgument`], including its leading `:`.
pub struct TranslateArgumentValue {
    /// The colon before the expression.
    pub colon_token: Token![:],
    /// The value. May be empty while the user types.
    pub expr: UnparsedExpr,
}

impl Parse for TranslateArgumentValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            colon_token: input.parse()?,
            expr: input.parse()?,
        })
    }
}

impl ParseOption for TranslateArgumentValue {
    fn peek(input: ParseStream) -> bool {
        // `name::x` is a path, not an argument value.
        input.peek(Token![:]) && !input.peek(Token![::])
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TranslateArgumentValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.colon_token.pretty_print(printer);
        if !self.expr.is_empty() {
            " ".pretty_print(printer);
            self.expr.pretty_print(printer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(input: &str) -> syn::Result<TranslateArguments> {
        syn::parse_str(input)
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    /// Extracts each argument's name and original value tokens.
    fn arguments(input: &str) -> Vec<(String, Option<String>)> {
        parse(input)
            .unwrap()
            .arguments
            .iter()
            .map(|argument| {
                (
                    argument.name.value(),
                    argument
                        .value
                        .as_ref()
                        .map(|value| value.expr.tokens.to_string()),
                )
            })
            .collect()
    }

    #[test]
    fn parses_nothing_without_a_comma() {
        assert!(parse("").unwrap().arguments.is_empty());
    }

    #[test]
    fn parses_values_and_shorthands() {
        assert_eq!(
            arguments(", count: n + 1, name,"),
            [
                ("count".to_owned(), Some(tokens("n + 1"))),
                ("name".to_owned(), None),
            ]
        );
    }

    #[test]
    fn accepts_a_value_being_typed() {
        assert_eq!(
            arguments(", count:, name: user"),
            [
                ("count".to_owned(), Some(String::new())),
                ("name".to_owned(), Some(tokens("user"))),
            ]
        );
    }

    /// Extracts the value expression for each argument.
    fn values(input: &str) -> Vec<String> {
        parse(input)
            .unwrap()
            .arguments
            .iter()
            .map(|argument| argument.value().to_string())
            .collect()
    }

    #[test]
    fn a_shorthand_is_its_own_value() {
        assert_eq!(
            values(", count: n + 1, name"),
            [tokens("n + 1"), tokens("name")]
        );
    }

    #[test]
    fn a_value_being_typed_is_a_compile_error() {
        let values = values(", count:, name");
        assert!(values[0].contains("compile_error"), "{}", values[0]);
        assert!(
            values[0].contains("expected a value for `count`"),
            "{}",
            values[0]
        );
        assert_eq!(values[1], tokens("name"));
    }

    #[test]
    fn rejects_an_argument_without_a_name() {
        assert!(parse(", = 5").is_err());
        assert!(parse(", 5").is_err());
    }

    #[test]
    fn rejects_a_path_after_the_name() {
        assert!(parse(", name::value").is_err());
    }

    #[test]
    fn rejects_arguments_without_a_comma() {
        assert!(parse("count: 5").is_err());
    }

    #[test]
    fn parses_names_that_are_not_rust_identifiers() {
        assert_eq!(
            arguments(r#", "userName": a, user-name: b, "x-1e": c"#),
            [
                ("userName".to_owned(), Some(tokens("a"))),
                ("user-name".to_owned(), Some(tokens("b"))),
                ("x-1e".to_owned(), Some(tokens("c"))),
            ]
        );
    }

    #[test]
    fn keeps_underscores_in_names() {
        assert_eq!(
            arguments(", user_name: a"),
            [("user_name".to_owned(), Some(tokens("a")))]
        );
    }

    #[test]
    fn a_raw_shorthand_passes_the_unraw_name() {
        let parsed = parse(", r#type").unwrap();
        let argument = &parsed.arguments[0];
        assert_eq!(argument.name.value(), "type");
        assert_eq!(argument.value().to_string(), tokens("r#type"));
    }

    #[test]
    fn rejects_a_shorthand_that_is_not_a_rust_identifier() {
        for input in [", user-name", r#", "name""#] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_a_name_passed_twice() {
        for input in [
            ", count: 1, count: 2",
            ", name, name: x",
            ", r#type, type: x",
            r#", user-name: a, "user-name": b"#,
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn parses_doc_comments_before_a_name() {
        let parsed = parse(", /// The minimum length.\n min: 8, /// The user.\n name").unwrap();
        for argument in &parsed.arguments {
            assert_eq!(argument.docs.attrs.len(), 1);
        }
    }
}
