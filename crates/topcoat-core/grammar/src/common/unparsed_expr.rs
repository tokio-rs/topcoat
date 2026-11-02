use proc_macro2::{Delimiter, Spacing, TokenStream, TokenTree};
use quote::ToTokens;
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

/// A Rust expression, kept as unparsed tokens.
///
/// Parsing stops at the first comma that is not part of the expression. The
/// expression may be incomplete or empty.
pub struct UnparsedExpr {
    /// The expression tokens. Empty when no expression was written.
    pub tokens: TokenStream,
}

impl UnparsedExpr {
    /// Checks whether no tokens were written.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}

impl Parse for UnparsedExpr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut tokens = TokenStream::new();
        let mut scanner = Scanner::default();
        while !input.is_empty() && (!scanner.ends_at_comma() || !input.peek(Token![,])) {
            let token: TokenTree = input.parse()?;
            scanner.push(&token);
            tokens.extend([token]);
        }
        Ok(Self { tokens })
    }
}

impl ToTokens for UnparsedExpr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.tokens.to_tokens(tokens);
    }
}

/// Formats the tokens as a Rust expression. Tokens that are not a complete
/// expression are copied as written.
#[cfg(feature = "pretty")]
impl crate::pretty::PrettyPrint for UnparsedExpr {
    fn pretty_print(&self, printer: &mut crate::pretty::Printer<'_>) {
        use syn::spanned::Spanned;

        use crate::pretty::TextMode;

        if self.tokens.is_empty() {
            return;
        }
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

/// Tracks expression tokens to decide which commas end the expression.
///
/// Commas in `::<...>` generics and a leading closure's signature stay inside the
/// expression. An unclosed `::<` or `|` keeps the rest of the input in the
/// expression so incomplete expressions can still be parsed.
#[derive(Default)]
struct Scanner {
    /// Current depth of angle brackets in `::<...>` generics.
    angle_depth: usize,
    /// Current position in a leading closure's signature.
    closure: ClosureState,
    /// The last two tokens, recorded only when they are punctuation.
    previous: [Option<(char, Spacing)>; 2],
}

impl Scanner {
    /// Checks whether a comma here would end the expression.
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

/// The scanner's position within a closure at the start of an expression.
#[derive(Default, Clone, Copy)]
enum ClosureState {
    /// At the start of the expression, possibly after `move` or `async`.
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
    use syn::punctuated::Punctuated;

    use super::*;

    /// Splits a comma-separated list into its expressions.
    fn split(input: &str) -> Vec<String> {
        syn::parse::Parser::parse_str(
            Punctuated::<UnparsedExpr, Token![,]>::parse_terminated,
            input,
        )
        .unwrap()
        .iter()
        .map(|expr| expr.tokens.to_string())
        .collect()
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn keeps_commas_inside_groups() {
        assert_eq!(
            split("max(a, b), [1, 2], (x, y)"),
            [tokens("max(a, b)"), tokens("[1, 2]"), tokens("(x, y)")]
        );
    }

    #[test]
    fn keeps_commas_inside_turbofish_generics() {
        assert_eq!(
            split("HashMap::<K, V>::new(), call::<Box<dyn Fn(u8) -> u8>, u8>(x)"),
            [
                tokens("HashMap::<K, V>::new()"),
                tokens("call::<Box<dyn Fn(u8) -> u8>, u8>(x)"),
            ]
        );
    }

    #[test]
    fn splits_after_comparisons() {
        assert_eq!(
            split("x < y, z > w"),
            [tokens("x < y"), tokens("z > w")]
        );
    }

    #[test]
    fn keeps_commas_inside_closure_signatures() {
        assert_eq!(
            split("move |a, b| a + b, |a, b| -> Result<A, B> { Ok(a) }, || 1, name"),
            [
                tokens("move |a, b| a + b"),
                tokens("|a, b| -> Result<A, B> { Ok(a) }"),
                tokens("|| 1"),
                tokens("name"),
            ]
        );
    }

    #[test]
    fn splits_after_a_logical_or() {
        assert_eq!(split("x || y, z"), [tokens("x || y"), tokens("z")]);
    }

    #[test]
    fn keeps_the_rest_in_an_unclosed_expression() {
        assert_eq!(
            split("HashMap::<String, name"),
            [tokens("HashMap::<String, name")]
        );
        assert_eq!(split("|a, name"), [tokens("|a, name")]);
    }

    #[test]
    fn accepts_a_missing_expression() {
        assert_eq!(split(", user"), [String::new(), tokens("user")]);
    }
}
