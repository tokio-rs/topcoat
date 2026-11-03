use proc_macro2::{Delimiter, Spacing, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote_spanned};
use syn::{
    Token,
    parse::{Parse, ParseStream},
    token::Brace,
};

/// A Rust expression, kept as unparsed tokens.
///
/// A complete expression ends where `syn` would end it, so `"active" if
/// selected` ends before the `if`. An incomplete expression, such as `user.`
/// while it is being typed, ends before the next `,`, `;` or `=>` outside of
/// any group instead, so the macro keeps parsing. An empty expression expands
/// to a compile error at the position where it was expected.
#[derive(Clone)]
pub struct Expr {
    /// The expression tokens. Empty when no expression was written.
    pub tokens: TokenStream,
    /// The position where the expression starts.
    pub span: Span,
}

impl Expr {
    /// Checks whether no tokens were written.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Parses an expression that is followed by a `{ ... }` block, like the
    /// condition of an `if` or the iterator of a `for` loop.
    ///
    /// As with [`syn::Expr::parse_without_eager_brace`], the block is not read as a
    /// struct literal. An incomplete expression also ends before the next
    /// `{ ... }` outside of any group.
    ///
    /// # Errors
    ///
    /// Returns an error if the input contains an invalid token tree.
    pub fn parse_without_eager_brace(input: ParseStream) -> syn::Result<Self> {
        Self::parse_with(input, syn::Expr::parse_without_eager_brace, true)
    }

    /// Takes the tokens `parse` accepts as an expression, or scans for the end
    /// of an incomplete expression when it accepts none.
    fn parse_with(
        input: ParseStream,
        parse: fn(ParseStream) -> syn::Result<syn::Expr>,
        stop_at_block: bool,
    ) -> syn::Result<Self> {
        let span = input.span();
        let mut tokens = TokenStream::new();
        let fork = input.fork();
        if parse(&fork).is_ok() {
            while input.cursor() != fork.cursor() {
                tokens.extend([input.parse::<TokenTree>()?]);
            }
        } else {
            let mut scanner = Scanner::default();
            while !input.is_empty() && !scanner.ends_before(input, stop_at_block) {
                let token: TokenTree = input.parse()?;
                scanner.push(&token);
                tokens.extend([token]);
            }
        }
        Ok(Self { tokens, span })
    }
}

impl Parse for Expr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_with(input, syn::Expr::parse, false)
    }
}

impl ToTokens for Expr {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        if self.tokens.is_empty() {
            quote_spanned! {self.span=> ::core::compile_error!("expected an expression") }
                .to_tokens(tokens);
        } else {
            self.tokens.to_tokens(tokens);
        }
    }
}

/// Formats the tokens as a Rust expression. Tokens that are not a complete
/// expression are copied as written.
#[cfg(feature = "pretty")]
impl crate::common::pretty::PrettyPrint for Expr {
    fn pretty_print(&self, printer: &mut crate::common::pretty::Printer<'_>) {
        use syn::spanned::Spanned;

        use crate::common::pretty::TextMode;

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

/// Finds the end of an incomplete expression.
///
/// Commas in `::<...>` generics and a leading closure's signature stay inside the
/// expression. An unclosed `::<` or `|` keeps the rest of the input in the
/// expression.
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
    /// Checks whether the expression ends before the next token in `input`: a
    /// `,`, `;` or `=>`, which cannot appear outside of a group in an
    /// expression, or a `{ ... }` block when `stop_at_block` is set.
    fn ends_before(&self, input: ParseStream, stop_at_block: bool) -> bool {
        let in_signature = self.angle_depth > 0
            || matches!(
                self.closure,
                ClosureState::Params | ClosureState::ReturnType
            );
        !in_signature
            && (input.peek(Token![,])
                || input.peek(Token![;])
                || input.peek(Token![=>])
                || (stop_at_block && input.peek(Brace)))
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
        syn::parse::Parser::parse_str(Punctuated::<Expr, Token![,]>::parse_terminated, input)
            .unwrap()
            .iter()
            .map(|expr| expr.tokens.to_string())
            .collect()
    }

    /// Parses the expression at the start of `input` with `parse`, ignoring
    /// the rest.
    fn first_with(parse: fn(ParseStream) -> syn::Result<Expr>, input: &str) -> String {
        syn::parse::Parser::parse_str(
            |input: ParseStream| {
                let expr = parse(input)?;
                input.parse::<TokenStream>()?;
                Ok(expr)
            },
            input,
        )
        .unwrap()
        .tokens
        .to_string()
    }

    fn first(input: &str) -> String {
        first_with(Expr::parse, input)
    }

    fn before_block(input: &str) -> String {
        first_with(Expr::parse_without_eager_brace, input)
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn ends_a_complete_expression_where_syn_does() {
        assert_eq!(
            split("max(a, b), HashMap::<K, V>::new(), |a, b| a + b"),
            [
                tokens("max(a, b)"),
                tokens("HashMap::<K, V>::new()"),
                tokens("|a, b| a + b"),
            ]
        );
        assert_eq!(first(r#""active" if selected"#), tokens(r#""active""#));
        assert_eq!(first(r#""A" src: local("A")"#), tokens(r#""A""#));
        assert_eq!(first("item.id => body"), tokens("item.id"));
    }

    #[test]
    fn ends_an_incomplete_expression_at_a_separator() {
        assert_eq!(split("user., name"), [tokens("user."), tokens("name")]);
        assert_eq!(first("a + ; b"), tokens("a +"));
        assert_eq!(first("user. => body"), tokens("user."));
    }

    #[test]
    fn keeps_commas_inside_an_incomplete_signature() {
        assert_eq!(
            split("|a, b| a., HashMap::<K, V>::new(."),
            [tokens("|a, b| a."), tokens("HashMap::<K, V>::new(.)")]
        );
        assert_eq!(
            split("HashMap::<String, name"),
            [tokens("HashMap::<String, name")]
        );
        assert_eq!(split("|a, name"), [tokens("|a, name")]);
    }

    #[test]
    fn ends_before_a_block() {
        assert_eq!(
            before_block("let Point { x, .. } = point { body }"),
            tokens("let Point { x, .. } = point")
        );
        assert_eq!(
            before_block("let Some(x) = user. { body }"),
            tokens("let Some(x) = user.")
        );
        assert_eq!(first("Point { x } + y"), tokens("Point { x } + y"));
    }

    #[test]
    fn accepts_a_missing_expression() {
        assert_eq!(split(", user"), [String::new(), tokens("user")]);
    }

    #[test]
    fn expands_a_missing_expression_to_a_compile_error() {
        let expr: Expr = syn::parse_str("").unwrap();
        let expansion: syn::ExprMacro = syn::parse2(expr.to_token_stream()).unwrap();
        assert_eq!(
            expansion.mac.path.segments.last().unwrap().ident,
            "compile_error"
        );
    }

    #[test]
    fn expands_a_written_expression_unchanged() {
        let expr: Expr = syn::parse_str("user.").unwrap();
        assert_eq!(expr.to_token_stream().to_string(), tokens("user."));
    }
}
