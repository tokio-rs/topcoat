mod scanner;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};
use syn::parse::{Parse, ParseStream};

use self::scanner::Scanner;

/// A Rust expression, kept as unparsed tokens.
///
/// The expression ends at the first token that cannot continue it, so
/// `"active" if selected` ends before the `if`. Only whole token trees are
/// read, never the inside of a group. Code that is still being typed follows
/// the same rule: `user., name` ends after the `.`, so the macro keeps parsing
/// the rest of its input. An empty expression expands to a compile error at
/// the position where it was expected.
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
    /// As with [`syn::Expr::parse_without_eager_brace`], the block is not read
    /// as a struct literal. It does not start a block expression either, so an
    /// empty condition ends before the block.
    ///
    /// # Errors
    ///
    /// Never returns an error. The signature matches other parse functions.
    pub fn parse_without_eager_brace(input: ParseStream) -> syn::Result<Self> {
        Self::parse_with(input, false)
    }

    /// Takes the tokens of the expression at the start of `input`.
    fn parse_with(input: ParseStream, allow_brace: bool) -> syn::Result<Self> {
        let span = input.span();
        input.step(|cursor| {
            let end = Scanner::new(allow_brace).expr(*cursor);
            let mut tokens = TokenStream::new();
            let mut next = *cursor;
            while next != end {
                let Some((token, after)) = next.token_tree() else {
                    break;
                };
                tokens.extend([token]);
                next = after;
            }
            Ok((Self { tokens, span }, end))
        })
    }
}

impl Parse for Expr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_with(input, true)
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

#[cfg(test)]
mod tests {
    use proc_macro2::TokenTree;
    use syn::{Token, punctuated::Punctuated};

    use super::*;

    /// Complete expressions covering every kind of operand and of token that
    /// continues an expression.
    const EXPRESSIONS: &[&str] = &[
        "x",
        "42",
        "-1",
        r#""text""#,
        "true",
        "_",
        "user.name.len()",
        "t.0.1",
        "r#type.r#match",
        "f(a, b)(c)",
        "v[0]",
        "fut.await?",
        "x.await.y",
        "a.b::<T>()",
        "a + b * c - d / e % f",
        "a == b && c != d || !e",
        "a << 2 >> 1 & b | c ^ d",
        "a < b",
        "a >= b",
        "a || b",
        "x = 1",
        "x <<= 1",
        "*x += 1",
        "&mut x",
        "&&x",
        "&raw const x",
        "&*x",
        "-(-x)",
        "a..b",
        "a..",
        "..b",
        "..",
        "a..=b",
        "x as u8",
        "x as i64 as f64",
        "x as u8 + 1",
        "x as Vec<u8>",
        "x as Vec<Vec<u8>>",
        "&x as *const u8",
        "x as fn(u8) -> u8",
        "x as &'static str",
        "x as <T as Tr>::Out",
        "x as Box<dyn Fn(u8) -> u8>",
        "Vec::<u8>::new()",
        "<Vec<u8> as Default>::default()",
        "::std::mem::take(x)",
        "iter.collect::<Vec<_>>()",
        "self.x",
        "Self::new()",
        "crate::f()",
        "super::f()",
        "Point { x: 1, y }",
        "Point::<u8> { x }",
        "Point { x }.x",
        r#"format!("{}", x)"#,
        "vec![1, 2]",
        "m! { a }",
        "(a, b)",
        "[a; 3]",
        "{ a }",
        "'a: { break 'a 1 }",
        "if a { b } else { c }",
        "if a { b } else if c { d } else { e }",
        "if let Some(x) = y { x } else { 0 }",
        "if a { b } else { c }.len()",
        "match x { _ => 1 }",
        "match x { _ => 1 } + 2",
        "loop { break 1 }",
        "while a { b }",
        "while let Some(x) = it.next() { f(x) }",
        "for x in xs { f(x) }",
        "for (a, b) in pairs.iter() { f(a) }",
        "'outer: loop {}",
        "unsafe { f() }",
        "async { 1 }",
        "async move { 1 }",
        "const { 1 }",
        "|| 1",
        "|a, b| a + b",
        "move |x| x.y",
        "|x: u8| -> u8 { x }",
        "|(a, b): (u8, u8)| a",
        "|&x| x",
        "Some(x).map(|y| y + 1).unwrap_or(0)",
        "return",
        "return x + 1",
        "break",
        "break 'a",
        "break 'a x",
        "continue",
        "continue 'a",
        "yield x",
    ];

    /// Conditions of an `if` or `while`, which may contain `let`.
    const CONDITIONS: &[&str] = &[
        "let Some(x) = y",
        "let Point { x, .. } = p",
        "let 0..=9 = n",
        "a && let Some(b) = c",
        "x == y",
        "f(Point { x })",
    ];

    /// Tokens that end an expression without being part of it.
    const ENDINGS: &[&str] = &[
        "",
        ", x",
        "; x",
        "=> x",
        " if c",
        " else c",
        " in c",
        r#" "text""#,
        ": u8",
    ];

    /// Returns the tokens that `parse` reads at the start of `input`, or
    /// `None` when it fails.
    fn read<T>(parse: fn(ParseStream) -> syn::Result<T>, input: &str) -> Option<String> {
        syn::parse::Parser::parse_str(
            |input: ParseStream| {
                let fork = input.fork();
                parse(&fork)?;
                let mut tokens = TokenStream::new();
                while input.cursor() != fork.cursor() {
                    tokens.extend([input.parse::<TokenTree>()?]);
                }
                input.parse::<TokenStream>()?;
                Ok(tokens.to_string())
            },
            input,
        )
        .ok()
    }

    /// Splits a comma-separated list into its expressions.
    fn split(input: &str) -> Vec<String> {
        syn::parse::Parser::parse_str(Punctuated::<Expr, Token![,]>::parse_terminated, input)
            .unwrap()
            .iter()
            .map(|expr| expr.tokens.to_string())
            .collect()
    }

    fn first(input: &str) -> String {
        read(Expr::parse, input).unwrap()
    }

    fn before_block(input: &str) -> String {
        read(Expr::parse_without_eager_brace, input).unwrap()
    }

    fn tokens(input: &str) -> String {
        input.parse::<TokenStream>().unwrap().to_string()
    }

    #[test]
    fn ends_a_complete_expression_where_syn_does() {
        for expr in EXPRESSIONS {
            assert_eq!(read(syn::Expr::parse, expr), Some(tokens(expr)), "{expr}");
            for ending in ENDINGS {
                let input = format!("{expr}{ending}");
                // An ending such as `if c` after `return` makes the input
                // invalid; only valid input has an end to compare.
                if let Some(expected) = read(syn::Expr::parse, &input) {
                    assert_eq!(first(&input), expected, "{input}");
                }
            }
        }
    }

    #[test]
    fn ends_a_condition_where_syn_does() {
        let conditions = EXPRESSIONS
            .iter()
            .filter(|expr| !expr.starts_with('{'))
            .chain(CONDITIONS);
        let body = tokens("{ body }");
        for condition in conditions {
            let input = format!("{condition} {body}");
            if let Some(expected) = read(syn::Expr::parse_without_eager_brace, &input) {
                // `syn` reads the body as the value of `return` or a range,
                // where a block never starts an operand here.
                let expected = expected
                    .strip_suffix(&body)
                    .map_or(&*expected, str::trim_end);
                assert_eq!(before_block(&input), expected, "{input}");
            }
        }
    }

    #[test]
    fn never_reads_past_a_comma_while_typing() {
        // Unclosed generics and closure parameters keep commas on purpose,
        // and a lifetime cannot be cut in two.
        let expressions = EXPRESSIONS
            .iter()
            .filter(|expr| !expr.contains(['<', '|', '\'']));
        for expr in expressions {
            let trees: Vec<TokenTree> = expr.parse::<TokenStream>().unwrap().into_iter().collect();
            for len in 1..trees.len() {
                let prefix: TokenStream = trees[..len].iter().cloned().collect();
                let input = format!("{prefix}, rest");
                let read: TokenStream = first(&input).parse().unwrap();
                assert!(read.into_iter().count() <= len, "{input}");
            }
        }
    }

    #[test]
    fn ends_incomplete_code_after_its_last_token() {
        assert_eq!(split("user., name"), [tokens("user."), tokens("name")]);
        assert_eq!(first("user. if selected"), tokens("user."));
        assert_eq!(first("user. => body"), tokens("user."));
        assert_eq!(first("a + ; b"), tokens("a +"));
        assert_eq!(first("x as , y"), tokens("x as"));
        assert_eq!(first("if c { a } else , y"), tokens("if c { a } else"));
        assert_eq!(first("|a, b| a., y"), tokens("|a, b| a."));
        assert_eq!(first("foo(user.), y"), tokens("foo(user.)"));
    }

    #[test]
    fn keeps_commas_in_unclosed_generics_and_closure_parameters() {
        assert_eq!(
            split("HashMap::<String, name"),
            [tokens("HashMap::<String, name")]
        );
        assert_eq!(first("|a, name; rest"), tokens("|a, name"));
    }

    #[test]
    fn ends_an_incomplete_condition_before_the_body() {
        assert_eq!(
            before_block("let Some(x) = user. { body }"),
            tokens("let Some(x) = user.")
        );
        assert_eq!(before_block("a == { body }"), tokens("a =="));
        assert_eq!(before_block("{ body }"), "");
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
