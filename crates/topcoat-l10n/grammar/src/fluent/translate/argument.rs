use proc_macro2::{TokenStream, TokenTree};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::ParseOption;

/// The arguments of a `t!` invocation, after the message.
///
/// The arguments are split at top-level commas. Each value is kept as the
/// tokens written up to the next comma, so a value that is still being typed
/// does not fail to parse. A comma at the top level of a value, such as in a
/// closure with several parameters, ends the value early; wrap the value in
/// parentheses to avoid that.
pub struct TranslateArguments {
    /// The comma between the message and its arguments, if any.
    pub comma_token: Option<Token![,]>,
    /// The arguments.
    pub arguments: Punctuated<TranslateArgument, Token![,]>,
}

impl TranslateArguments {
    /// Returns whether every argument is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.arguments.iter().all(TranslateArgument::is_complete)
    }
}

impl Parse for TranslateArguments {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let comma_token: Option<Token![,]> = input.parse()?;
        let mut arguments = Punctuated::new();
        if comma_token.is_some() {
            while !input.is_empty() {
                arguments.push_value(input.parse()?);
                if input.is_empty() {
                    break;
                }
                arguments.push_punct(input.parse()?);
            }
        }
        Ok(Self {
            comma_token,
            arguments,
        })
    }
}

/// An argument of a message, either `name = value` or the shorthand `name`
/// for a variable of the same name.
pub struct TranslateArgument {
    /// The name of the message variable.
    pub name: Ident,
    /// The value of the variable, if not written as a shorthand.
    pub value: Option<TranslateArgumentValue>,
}

impl TranslateArgument {
    /// Returns whether the argument is complete: a shorthand, or an `=`
    /// followed by a value.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.value
            .as_ref()
            .is_none_or(|value| !value.tokens.is_empty())
    }
}

impl Parse for TranslateArgument {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            value: input.call(TranslateArgumentValue::parse_option)?,
        })
    }
}

/// The `= value` part of a [`TranslateArgument`].
pub struct TranslateArgumentValue {
    /// The `=` before the value.
    pub eq_token: Token![=],
    /// The tokens of the value up to the next top-level comma, as written.
    /// Empty while the value is being typed.
    pub tokens: TokenStream,
}

impl Parse for TranslateArgumentValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            eq_token: input.parse()?,
            tokens: {
                let mut tokens = TokenStream::new();
                while !input.is_empty() && !input.peek(Token![,]) {
                    tokens.extend([input.parse::<TokenTree>()?]);
                }
                tokens
            },
        })
    }
}

impl ParseOption for TranslateArgumentValue {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![=])
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

    /// The name and value tokens of each argument.
    fn arguments(input: &str) -> Vec<(String, Option<String>)> {
        parse(input)
            .unwrap()
            .arguments
            .iter()
            .map(|argument| {
                (
                    argument.name.to_string(),
                    argument
                        .value
                        .as_ref()
                        .map(|value| value.tokens.to_string()),
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
            arguments(", count = n + 1, name,"),
            [
                ("count".to_owned(), Some(tokens("n + 1"))),
                ("name".to_owned(), None),
            ]
        );
    }

    #[test]
    fn keeps_commas_inside_groups_in_the_value() {
        assert_eq!(
            arguments(", count = max(a, b), items = [1, 2]"),
            [
                ("count".to_owned(), Some(tokens("max(a, b)"))),
                ("items".to_owned(), Some(tokens("[1, 2]"))),
            ]
        );
    }

    #[test]
    fn accepts_a_value_being_typed() {
        let parsed = parse(", count =, name = user").unwrap();
        assert!(!parsed.is_complete());
        assert_eq!(
            arguments(", count =, name = user"),
            [
                ("count".to_owned(), Some(String::new())),
                ("name".to_owned(), Some(tokens("user"))),
            ]
        );
    }

    #[test]
    fn is_complete_with_values_and_shorthands() {
        assert!(parse(", count = 5, name").unwrap().is_complete());
    }

    #[test]
    fn rejects_an_argument_without_a_name() {
        assert!(parse(", = 5").is_err());
        assert!(parse(", 5").is_err());
    }

    #[test]
    fn rejects_arguments_without_a_comma() {
        assert!(parse("count = 5").is_err());
    }
}
