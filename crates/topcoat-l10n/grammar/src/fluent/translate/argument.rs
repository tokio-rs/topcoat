use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use syn::{
    Ident, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::ParseOption;

/// The arguments of a `t!` invocation, after the message.
///
/// The arguments are split at commas. Each value is kept as the tokens written
/// up to the next comma that separates arguments, so a value that is still
/// being typed does not fail to parse. Commas inside brackets, turbofish
/// generics such as `HashMap::<K, V>::new()`, and the parameters and return
/// type of a closure that starts the value belong to the value. Any other comma
/// ends the value, such as in `<HashMap<K, V> as Default>::default()`; wrap such
/// a value in parentheses.
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
    /// The tokens of the value up to the comma that ends it, as written. Empty
    /// while the value is being typed.
    pub tokens: TokenStream,
}

impl Parse for TranslateArgumentValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            eq_token: input.parse()?,
            tokens: {
                let mut tokens = TokenStream::new();
                let mut scanner = ValueScanner::default();
                while !input.is_empty() && (!scanner.ends_at_comma() || !input.peek(Token![,])) {
                    let token: TokenTree = input.parse()?;
                    scanner.push(&token);
                    tokens.extend([token]);
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

/// Tracks the tokens of an argument value to tell whether a comma ends it.
///
/// A comma does not end the value inside turbofish generics, or inside the
/// parameters or return type of a closure that starts the value. The scanner
/// only looks at opening tokens, so an unclosed `::<` or `|` keeps the rest of
/// the input in the value while it is being typed.
#[derive(Default)]
struct ValueScanner {
    /// The nesting depth of `<` inside turbofish generics.
    angle_depth: usize,
    /// The progress through a closure that starts the value.
    closure: ClosureState,
    /// The previous two tokens, if they were punctuation.
    previous: [Option<(char, Spacing)>; 2],
}

impl ValueScanner {
    /// Returns whether a comma at the current position ends the value.
    fn ends_at_comma(&self) -> bool {
        self.angle_depth == 0
            && !matches!(
                self.closure,
                ClosureState::Params | ClosureState::ReturnType
            )
    }

    /// Advances the scanner past `token`.
    fn push(&mut self, token: &TokenTree) {
        let punct = match token {
            TokenTree::Punct(punct) => Some((punct.as_char(), punct.spacing())),
            _ => None,
        };
        let char = punct.map(|(char, _)| char);
        let after_arrow = self.previous[1] == Some(('-', Spacing::Joint));

        self.closure = match self.closure {
            ClosureState::Start => match token {
                TokenTree::Ident(ident) if ident == "move" || ident == "async" => {
                    ClosureState::Start
                }
                _ if char == Some('|') => ClosureState::Params,
                _ => ClosureState::None,
            },
            ClosureState::Params if char == Some('|') => ClosureState::AfterParams,
            ClosureState::AfterParams if char == Some('-') => ClosureState::AfterParams,
            ClosureState::AfterParams if char == Some('>') && after_arrow => {
                ClosureState::ReturnType
            }
            ClosureState::AfterParams => ClosureState::None,
            ClosureState::ReturnType => match token {
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    ClosureState::None
                }
                _ => ClosureState::ReturnType,
            },
            state => state,
        };

        if !matches!(
            self.closure,
            ClosureState::Params | ClosureState::ReturnType
        ) {
            // The second `:` of `::<` is joint with the `<`, so only the first
            // one's spacing tells a path separator apart from two colons.
            let after_path_separator = self.previous[0] == Some((':', Spacing::Joint))
                && matches!(self.previous[1], Some((':', _)));
            match char {
                Some('<') if after_path_separator || self.angle_depth > 0 => {
                    self.angle_depth += 1;
                }
                Some('>') if self.angle_depth > 0 && !after_arrow => self.angle_depth -= 1,
                _ => {}
            }
        }

        self.previous = [self.previous[1], punct];
    }
}

/// The progress through a closure at the start of an argument value.
#[derive(Default, Clone, Copy)]
enum ClosureState {
    /// No token other than `move` or `async` has been seen yet.
    #[default]
    Start,
    /// Inside the closure's parameters, between the `|`s.
    Params,
    /// After the closure's parameters, before its return type or body.
    AfterParams,
    /// Inside the closure's return type, before its block body.
    ReturnType,
    /// The value is not a closure, or the closure's signature has ended.
    None,
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
    fn keeps_commas_inside_turbofish_generics() {
        assert_eq!(
            arguments(", map = HashMap::<K, V>::new(), f = call::<Box<dyn Fn(u8) -> u8>, u8>(x)"),
            [
                ("map".to_owned(), Some(tokens("HashMap::<K, V>::new()"))),
                (
                    "f".to_owned(),
                    Some(tokens("call::<Box<dyn Fn(u8) -> u8>, u8>(x)"))
                ),
            ]
        );
    }

    #[test]
    fn splits_after_comparisons() {
        assert_eq!(
            arguments(", a = x < y, b = z > w"),
            [
                ("a".to_owned(), Some(tokens("x < y"))),
                ("b".to_owned(), Some(tokens("z > w"))),
            ]
        );
    }

    #[test]
    fn keeps_commas_inside_closure_signatures() {
        assert_eq!(
            arguments(
                ", f = move |a, b| a + b, g = |a, b| -> Result<A, B> { Ok(a) }, h = || 1, name"
            ),
            [
                ("f".to_owned(), Some(tokens("move |a, b| a + b"))),
                (
                    "g".to_owned(),
                    Some(tokens("|a, b| -> Result<A, B> { Ok(a) }"))
                ),
                ("h".to_owned(), Some(tokens("|| 1"))),
                ("name".to_owned(), None),
            ]
        );
    }

    #[test]
    fn splits_after_a_logical_or() {
        assert_eq!(
            arguments(", a = x || y, b = z"),
            [
                ("a".to_owned(), Some(tokens("x || y"))),
                ("b".to_owned(), Some(tokens("z"))),
            ]
        );
    }

    #[test]
    fn keeps_the_rest_in_an_unclosed_value() {
        assert_eq!(
            arguments(", map = HashMap::<String, name"),
            [("map".to_owned(), Some(tokens("HashMap::<String, name")))]
        );
        assert_eq!(
            arguments(", f = |a, name"),
            [("f".to_owned(), Some(tokens("|a, name")))]
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
