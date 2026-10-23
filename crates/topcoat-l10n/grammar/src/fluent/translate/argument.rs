use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};
use topcoat_core_grammar::ParseOption;

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

/// A named message argument, written as `name = value` or just `name` to use a Rust
/// variable with that name.
///
/// Names identify Fluent variables, so `user_name` supplies `$user_name`. Use a string
/// literal for names that are not Rust identifiers, such as `"user-name" = name`. The
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
    /// An incomplete value produces a compile error at `=` while preserving the
    /// argument's place in the expansion.
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

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TranslateArgument {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.docs.pretty_print(printer);
        self.name.pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

/// An explicit value in a [`TranslateArgument`], including its leading `=`.
///
/// Stores tokens up to the next argument separator, even if the expression is
/// incomplete. Commas inside brackets, `::<...>` generics or a leading closure's
/// signature stay in the value. Other commas end it. Wrap expressions such as
/// `<HashMap<K, V> as Default>::default()` in parentheses to keep their commas inside
/// the value.
pub struct TranslateArgumentValue {
    /// The assignment sign before the expression.
    pub eq_token: Token![=],
    /// Original value tokens up to the argument separator. May be empty while the user
    /// types.
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

/// Formats the value as a Rust expression. A value that is not a complete expression
/// is copied as written.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for TranslateArgumentValue {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        use topcoat_core_grammar::pretty::TextMode;

        " ".pretty_print(printer);
        self.eq_token.pretty_print(printer);
        if self.tokens.is_empty() {
            return;
        }
        " ".pretty_print(printer);
        if let Ok(expr) = syn::parse2::<syn::Expr>(self.tokens.clone()) {
            expr.pretty_print(printer);
        } else {
            let span = self.tokens.span();
            let text = span
                .source_text()
                .unwrap_or_else(|| self.tokens.to_string());
            printer.move_cursor(span.start());
            printer.scan_text(text.into(), TextMode::Always);
            printer.move_cursor(span.end());
            printer.skip_trivia();
        }
    }
}

/// Tracks argument tokens to decide which commas separate arguments.
///
/// Commas in `::<...>` generics and a leading closure's signature stay inside the
/// value. An unclosed `::<` or `|` keeps the rest of the input in the value so
/// incomplete expressions can still be parsed.
#[derive(Default)]
struct ValueScanner {
    /// Current depth of angle brackets in `::<...>` generics.
    angle_depth: usize,
    /// Current position in a leading closure's signature.
    closure: ClosureState,
    /// The last two tokens, recorded only when they are punctuation.
    previous: [Option<(char, Spacing)>; 2],
}

impl ValueScanner {
    /// Checks whether a comma here would end the argument.
    fn ends_at_comma(&self) -> bool {
        self.angle_depth == 0
            && !matches!(
                self.closure,
                ClosureState::Params | ClosureState::ReturnType
            )
    }

    /// Updates the scanner after reading `token`.
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

/// The scanner's position within a closure at the start of a value.
#[derive(Default, Clone, Copy)]
enum ClosureState {
    /// At the start of the value, possibly after `move` or `async`.
    #[default]
    Start,
    /// Reading parameters between the opening and closing `|`.
    Params,
    /// Finished reading parameters, but not yet in the return type or body.
    AfterParams,
    /// Reading a return type before the closure's block.
    ReturnType,
    /// Outside a closure signature.
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
