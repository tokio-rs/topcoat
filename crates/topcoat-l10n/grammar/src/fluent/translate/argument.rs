use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::ParseOption;

use crate::fluent::{DocComments, FluentIdent, FluentIdentToken};

/// The arguments of a `t!` invocation, after the key.
///
/// Each name may only be passed once.
pub struct TranslateArguments {
    /// The comma between the key and the arguments, if any.
    pub comma_token: Option<Token![,]>,
    /// The arguments.
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

/// An argument of a message, either `name = value` or the shorthand `name`
/// for a variable of the same name.
///
/// The name is the name of the Fluent variable, so `user_name` passes
/// `$user_name`. A name that is not a Rust identifier can be written as a
/// string literal, such as `"user-name" = name`. The shorthand only accepts a
/// Rust identifier, and a raw identifier such as `r#type` passes `$type`.
pub struct TranslateArgument {
    /// The doc comments of the argument.
    pub docs: DocComments,
    /// The name of the Fluent variable.
    pub name: FluentIdent,
    /// The value of the variable, if not written as a shorthand.
    pub value: Option<TranslateArgumentValue>,
}

impl TranslateArgument {
    /// Returns the expression of the argument: the written value, or the name
    /// for a shorthand.
    ///
    /// A value that is still being typed becomes a compile error at its `=`,
    /// so that the argument keeps its place in the expansion.
    #[must_use]
    pub fn value(&self) -> TokenStream {
        match &self.value {
            None => self.name.to_token_stream(),
            Some(value) if value.tokens.is_empty() => syn::Error::new(
                value.eq_token.span,
                format!("expected a value for `{}` after `=`", self.name.value()),
            )
            .to_compile_error(),
            Some(value) => value.tokens.clone(),
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
                    "the shorthand only accepts a Rust identifier; write `{} = value`",
                    argument.name.to_token_stream()
                ),
            ));
        }
        Ok(argument)
    }
}

/// The `= value` part of a [`TranslateArgument`].
///
/// The value is kept as the tokens written up to the next comma that
/// separates arguments, so a value that is still being typed does not fail to
/// parse. Commas inside brackets, turbofish generics such as
/// `HashMap::<K, V>::new()`, and the parameters and return type of a closure
/// that starts the value belong to the value. Any other comma ends the value,
/// such as in `<HashMap<K, V> as Default>::default()`; wrap such a value in
/// parentheses.
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
                    argument.name.value(),
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
        assert_eq!(
            arguments(", count =, name = user"),
            [
                ("count".to_owned(), Some(String::new())),
                ("name".to_owned(), Some(tokens("user"))),
            ]
        );
    }

    /// The expression of each argument.
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
            values(", count = n + 1, name"),
            [tokens("n + 1"), tokens("name")]
        );
    }

    #[test]
    fn a_value_being_typed_is_a_compile_error() {
        let values = values(", count =, name");
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
    fn rejects_arguments_without_a_comma() {
        assert!(parse("count = 5").is_err());
    }

    #[test]
    fn parses_names_that_are_not_rust_identifiers() {
        assert_eq!(
            arguments(r#", "userName" = a, user-name = b, "x-1e" = c"#),
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
            arguments(", user_name = a"),
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
            ", count = 1, count = 2",
            ", name, name = x",
            ", r#type, type = x",
            r#", user-name = a, "user-name" = b"#,
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn parses_doc_comments_before_a_name() {
        let parsed = parse(", /// The minimum length.\n min = 8, /// The user.\n name").unwrap();
        for argument in &parsed.arguments {
            assert_eq!(argument.docs.attrs.len(), 1);
        }
    }
}
