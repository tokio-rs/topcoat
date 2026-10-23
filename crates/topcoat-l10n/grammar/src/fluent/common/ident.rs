use std::ops::Range;

use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use syn::{
    Ident, LitFloat, LitInt, LitStr, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
};

/// A name used by Fluent for a message, attribute or variable.
///
/// Write the name as tokens, such as `password-field`, or as a string literal when Rust
/// cannot tokenize it, such as `"item-1e"`. Names must start with an ASCII letter. The
/// remaining characters may be ASCII letters, digits, `_` or `-`.
///
/// ```text
/// password-field
/// step-2a
/// type
/// "item-1e"
/// ```
pub enum FluentIdent {
    /// An identifier built from names and numbers separated by hyphens.
    Tokens(Vec<FluentIdentToken>),
    /// An identifier supplied as a string literal.
    Str(LitStr),
}

impl FluentIdent {
    /// Returns the identifier as text.
    ///
    /// Removes spaces between tokens and the `r#` prefix on raw identifiers. For
    /// example, `password - r#type` becomes `password-type`.
    #[must_use]
    pub fn value(&self) -> String {
        match self {
            Self::Tokens(tokens) => tokens.iter().map(FluentIdentToken::text).collect(),
            Self::Str(lit) => lit.value(),
        }
    }

    /// Rejects suffixes on string literals, such as `x` in `"a"x`.
    pub(crate) fn check_str_suffix(lit: &LitStr) -> syn::Result<()> {
        if lit.suffix().is_empty() {
            Ok(())
        } else {
            Err(syn::Error::new(
                lit.span(),
                "a Fluent identifier cannot have a suffix",
            ))
        }
    }

    /// Checks whether the byte range `range` in the literal's `value` is a valid Fluent
    /// identifier.
    pub(crate) fn check_str_part(
        lit: &LitStr,
        value: &str,
        range: Range<usize>,
    ) -> syn::Result<()> {
        if range.is_empty() {
            return Err(syn::Error::new(
                lit.span(),
                "expected a Fluent identifier, found an empty string",
            ));
        }
        match InvalidChar::find(&value[range.clone()], true) {
            Some(mut invalid) => {
                invalid.offset += range.start;
                Err(syn::Error::new(
                    invalid.span_in(lit, value),
                    invalid.message(),
                ))
            }
            None => Ok(()),
        }
    }
}

impl Parse for FluentIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(LitStr) {
            let lit: LitStr = input.parse()?;
            Self::check_str_suffix(&lit)?;
            let value = lit.value();
            Self::check_str_part(&lit, &value, 0..value.len())?;
            return Ok(Self::Str(lit));
        }
        if !lookahead.peek(Ident::peek_any) {
            return Err(lookahead.error());
        }

        let mut tokens = vec![FluentIdentToken::Ident(input.call(Ident::parse_any)?)];
        while input.peek(Token![-]) {
            tokens.push(FluentIdentToken::Hyphen(input.parse()?));
            if input.peek(Ident::peek_any) {
                tokens.push(FluentIdentToken::Ident(input.call(Ident::parse_any)?));
            } else if input.peek(LitInt) {
                tokens.push(FluentIdentToken::Int(input.parse()?));
            } else if input.peek(LitFloat) {
                tokens.push(FluentIdentToken::Float(input.parse()?));
            }
        }
        for (index, token) in tokens.iter().enumerate() {
            if let Some(invalid) = InvalidChar::find(&token.text(), index == 0) {
                return Err(syn::Error::new_spanned(token, invalid.message()));
            }
        }
        Ok(Self::Tokens(tokens))
    }
}

/// Writes the identifier's original tokens.
impl ToTokens for FluentIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Tokens(parts) => {
                for part in parts {
                    part.to_tokens(tokens);
                }
            }
            Self::Str(lit) => lit.to_tokens(tokens),
        }
    }
}

/// Prints the identifier without spaces between its tokens, as in `password-field`.
#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FluentIdent {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Tokens(tokens) => tokens.pretty_print(printer),
            Self::Str(lit) => lit.pretty_print(printer),
        }
    }
}

/// One token in a [`FluentIdent`].
pub enum FluentIdentToken {
    /// A Rust identifier or keyword, such as `password`, `type` or `r#type`.
    Ident(Ident),
    /// The `-` separator.
    Hyphen(Token![-]),
    /// An integer token, possibly with a suffix. For example, Rust reads `2a` in
    /// `step-2a` as `2` with suffix `a`.
    Int(LitInt),
    /// A floating-point token, such as `1e5` in `x-1e5`.
    Float(LitFloat),
}

impl FluentIdentToken {
    /// Returns the text this token contributes to the identifier.
    fn text(&self) -> String {
        match self {
            Self::Ident(ident) => ident.unraw().to_string(),
            Self::Hyphen(_) => "-".to_owned(),
            Self::Int(lit) => lit.to_string(),
            Self::Float(lit) => lit.to_string(),
        }
    }
}

impl ToTokens for FluentIdentToken {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::Hyphen(hyphen) => hyphen.to_tokens(tokens),
            Self::Int(lit) => lit.to_tokens(tokens),
            Self::Float(lit) => lit.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for FluentIdentToken {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Ident(ident) => ident.pretty_print(printer),
            Self::Hyphen(hyphen) => hyphen.pretty_print(printer),
            Self::Int(lit) => lit.pretty_print(printer),
            Self::Float(lit) => lit.pretty_print(printer),
        }
    }
}

/// A character that is invalid at its position in a Fluent identifier.
struct InvalidChar {
    /// The character's byte offset in the input text or string literal value.
    offset: usize,
    /// The invalid character.
    char: char,
    /// Whether this is the first character of the identifier.
    first: bool,
}

impl InvalidChar {
    /// Finds the first invalid character in `text`. Set `first` when the text starts
    /// the identifier.
    fn find(text: &str, first: bool) -> Option<Self> {
        text.char_indices().find_map(|(offset, char)| {
            let first = first && offset == 0;
            let valid = if first {
                char.is_ascii_alphabetic()
            } else {
                char.is_ascii_alphanumeric() || char == '_' || char == '-'
            };
            (!valid).then_some(Self {
                offset,
                char,
                first,
            })
        })
    }

    /// Finds the character's source span in `lit`, using its decoded `value`.
    ///
    /// Falls back to the whole literal if the character cannot be located in the source
    /// or the compiler cannot return a smaller span. Escapes can require this fallback.
    fn span_in(&self, lit: &LitStr, value: &str) -> Span {
        let token = lit.token();
        let source = token.to_string();
        let unescaped = source
            .strip_prefix('"')
            .and_then(|source| source.strip_suffix('"'))
            == Some(value);
        let start = self.offset + 1;
        unescaped
            .then(|| token.subspan(start..start + self.char.len_utf8()))
            .flatten()
            .unwrap_or_else(|| lit.span())
    }

    /// Describes why this character is invalid.
    fn message(&self) -> String {
        let char = self.char;
        if self.first {
            format!("a Fluent identifier must start with an ASCII letter, found `{char}`")
        } else {
            format!(
                "a Fluent identifier can only contain ASCII letters, digits, `_` and `-`, \
                 found `{char}`"
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use syn::parse::Parser;

    use super::*;

    fn parse(input: &str) -> syn::Result<FluentIdent> {
        syn::parse_str(input)
    }

    fn value(input: &str) -> String {
        parse(input).unwrap().value()
    }

    /// Reads the leading identifier from `input`, returning its text and the tokens
    /// left over.
    fn parse_prefix(input: &str) -> (String, String) {
        let parser =
            |input: ParseStream| Ok((input.parse::<FluentIdent>()?, input.parse::<TokenStream>()?));
        let (ident, rest) = parser.parse_str(input).unwrap();
        (ident.value(), rest.to_string())
    }

    #[test]
    fn joins_tokens_as_written() {
        for input in [
            "title",
            "password-field",
            "a-b-c",
            "step-2a",
            "x-1e5",
            "x-0x1f",
            "x-1_000",
            "a--b",
            "trailing-",
            "camelCase",
            "snake_case",
            "a-_b",
            "x-true",
        ] {
            assert_eq!(value(input), input);
        }
    }

    #[test]
    fn accepts_keywords() {
        assert_eq!(value("type"), "type");
        assert_eq!(value("fn-mut"), "fn-mut");
    }

    #[test]
    fn removes_the_prefix_of_raw_identifiers() {
        assert_eq!(value("r#type"), "type");
        assert_eq!(value("password-r#type"), "password-type");
    }

    #[test]
    fn ignores_whitespace_around_hyphens() {
        assert_eq!(value("password - field"), "password-field");
    }

    #[test]
    fn stops_at_the_end_of_the_identifier() {
        for (input, ident, rest) in [
            (
                "password-field.placeholder",
                "password-field",
                ". placeholder",
            ),
            ("count = 5", "count", "= 5"),
            ("greeting, name", "greeting", ", name"),
            ("trailing-, name", "trailing-", ", name"),
            ("a b", "a", "b"),
            ("a-2 b", "a-2", "b"),
        ] {
            let rest = rest.parse::<TokenStream>().unwrap().to_string();
            assert_eq!(parse_prefix(input), (ident.to_owned(), rest), "{input}");
        }
    }

    #[test]
    fn accepts_identifiers_that_rust_cannot_lex_in_a_string() {
        for value in ["item-1e", "x-1e-b", "x-0x", "x-0b2", "x-0o9"] {
            assert_eq!(parse(&format!("{value:?}")).unwrap().value(), value);
        }
    }

    #[test]
    fn emits_the_identifier_as_written() {
        for input in ["password - r#type-2a", r#""item-1e""#] {
            assert_eq!(
                parse(input).unwrap().to_token_stream().to_string(),
                input.parse::<TokenStream>().unwrap().to_string()
            );
        }
    }

    #[test]
    fn rejects_invalid_tokens() {
        for input in ["größe", "a-größe", "_a", "x-1.5", "a-'c'"] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_invalid_strings() {
        for input in [
            r#""""#,
            r#""1a""#,
            r#""-a""#,
            r#""_a""#,
            r#""a.b""#,
            r#""a b""#,
            r#""größe""#,
            r#""a"suffix"#,
        ] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_input_that_does_not_start_an_identifier() {
        for input in ["", "-a", "2a", "'a'"] {
            assert!(parse(input).is_err(), "{input}");
        }
    }

    #[test]
    fn points_at_the_invalid_character_of_a_string() {
        let error = parse(r#""ab.c""#).err().unwrap();
        // The `.` follows the quote and two letters.
        assert_eq!(error.span().start().column, 3);
    }

    #[test]
    fn points_at_a_string_with_escapes_as_a_whole() {
        let error = parse(r#""\x41.b""#).err().unwrap();
        assert_eq!(error.span().start().column, 0);
    }
}
