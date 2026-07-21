use std::fmt::{self, Display, Write};

use proc_macro2::{LineColumn, Span, TokenStream};
use quote::ToTokens;
use syn::{
    Ident, LitInt, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

use topcoat_core_grammar::ParseOption;

/// An HTML identifier: a sequence of identifier segments joined by `-`, `:`,
/// or `.` with no intervening whitespace. Covers names like `data-foo`,
/// `aria-label`, `xmlns:xlink`, or `class.active` that are valid in HTML but
/// not valid Rust identifiers.
#[derive(Debug, PartialEq)]
pub struct HtmlIdent {
    pub first: Ident,
    pub rest: Vec<HtmlIdentSegment>,
}

impl HtmlIdent {
    /// The source span covering the identifier. Falls back to the first
    /// segment's span when the underlying [`Span::join`] is unavailable (i.e.
    /// on stable Rust outside of `proc_macro2`'s fallback mode).
    #[must_use]
    pub fn span(&self) -> Span {
        let first = self.first.span();
        match self.rest.last() {
            Some(segment) => first.join(segment.part.span()).unwrap_or(first),
            None => first,
        }
    }

    /// Parses an [`HtmlIdent`] that only allows `-` as a separator. Used for
    /// HTML element names, where `:` and `.` would tear apart adjacent
    /// attribute syntax like `:value` or `class.active`.
    ///
    /// # Errors
    ///
    /// Returns an error if the input does not begin with a valid identifier, or
    /// if a `-` separator is not adjacent to the surrounding identifier segments.
    pub fn parse_dash_only(input: ParseStream) -> syn::Result<Self> {
        Self::parse_inner(input, false)
    }

    fn parse_inner(input: ParseStream, allow_colon_dot: bool) -> syn::Result<Self> {
        let first = Ident::parse_any(input)?;
        let mut rest = Vec::new();
        let mut prev_end = first.span().end();

        loop {
            // Bail out on multi-character punctuation that starts with one of
            // our separators (`::`, `..`) so we don't tear apart a path or
            // range expression that happens to follow the identifier.
            if input.peek(Token![::]) || input.peek(Token![..]) {
                break;
            }

            let separator = if input.peek(Token![-]) {
                HtmlIdentSeparator::Dash(input.parse()?)
            } else if allow_colon_dot && input.peek(Token![:]) {
                HtmlIdentSeparator::Colon(input.parse()?)
            } else if allow_colon_dot && input.peek(Token![.]) {
                HtmlIdentSeparator::Dot(input.parse()?)
            } else {
                break;
            };

            let separator_span = separator.span();
            if !is_adjacent(prev_end, separator_span.start()) {
                return Err(syn::Error::new(
                    separator_span,
                    "whitespace is not allowed inside an HTML identifier",
                ));
            }

            let part = HtmlIdentPart::parse(input)?;
            if !is_adjacent(separator_span.end(), part.span().start()) {
                return Err(syn::Error::new(
                    part.span(),
                    "whitespace is not allowed inside an HTML identifier",
                ));
            }

            prev_end = part.span().end();
            rest.push(HtmlIdentSegment { separator, part });
        }

        Ok(Self { first, rest })
    }
}

/// A trailing `<sep><part>` segment of an [`HtmlIdent`].
#[derive(Debug, PartialEq)]
pub struct HtmlIdentSegment {
    pub separator: HtmlIdentSeparator,
    pub part: HtmlIdentPart,
}

/// The character joining two segments of an [`HtmlIdent`].
#[derive(Debug, PartialEq)]
pub enum HtmlIdentSeparator {
    Dash(Token![-]),
    Colon(Token![:]),
    Dot(Token![.]),
}

impl HtmlIdentSeparator {
    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Dash(token) => token.span(),
            Self::Colon(token) => token.span(),
            Self::Dot(token) => token.span(),
        }
    }

    fn as_char(&self) -> char {
        match self {
            Self::Dash(_) => '-',
            Self::Colon(_) => ':',
            Self::Dot(_) => '.',
        }
    }
}

/// The identifier portion of an [`HtmlIdentSegment`]. Usually a Rust-style
/// identifier, but HTML attribute names may also carry a bare number such as
/// Alpine AJAX's `x-target.422` status-code modifiers -- including wildcard
/// forms like `4xx` -- which are not valid Rust identifiers.
#[derive(Debug, PartialEq)]
pub enum HtmlIdentPart {
    Ident(Ident),
    Int(LitInt),
}

impl HtmlIdentPart {
    /// Parses a segment identifier: either a Rust identifier (including
    /// keywords) or an integer literal, which the Rust lexer produces for
    /// segments that begin with a digit such as `422` or `4xx`.
    ///
    /// # Errors
    ///
    /// Returns an error if the input begins with neither of those.
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitInt) {
            Ok(Self::Int(input.parse()?))
        } else {
            Ok(Self::Ident(Ident::parse_any(input)?))
        }
    }

    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            Self::Ident(ident) => ident.span(),
            Self::Int(int) => int.span(),
        }
    }
}

impl Display for HtmlIdentPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ident(ident) => Display::fmt(ident, f),
            Self::Int(int) => Display::fmt(int, f),
        }
    }
}

impl ToTokens for HtmlIdentPart {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Ident(ident) => ident.to_tokens(tokens),
            Self::Int(int) => int.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for HtmlIdentPart {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        match self {
            Self::Ident(ident) => ident.pretty_print(printer),
            Self::Int(int) => int.pretty_print(printer),
        }
    }
}

impl Display for HtmlIdent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.first, f)?;
        for segment in &self.rest {
            f.write_char(segment.separator.as_char())?;
            Display::fmt(&segment.part, f)?;
        }
        Ok(())
    }
}

impl Parse for HtmlIdent {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Self::parse_inner(input, true)
    }
}

impl ParseOption for HtmlIdent {
    fn peek(input: ParseStream) -> bool {
        input.peek(Ident::peek_any)
    }
}

impl ToTokens for HtmlIdent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.first.to_tokens(tokens);
        for segment in &self.rest {
            match &segment.separator {
                HtmlIdentSeparator::Dash(t) => t.to_tokens(tokens),
                HtmlIdentSeparator::Colon(t) => t.to_tokens(tokens),
                HtmlIdentSeparator::Dot(t) => t.to_tokens(tokens),
            }
            segment.part.to_tokens(tokens);
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_core_grammar::pretty::PrettyPrint for HtmlIdent {
    fn pretty_print(&self, printer: &mut topcoat_core_grammar::pretty::Printer<'_>) {
        self.first.pretty_print(printer);
        for segment in &self.rest {
            match &segment.separator {
                HtmlIdentSeparator::Dash(token) => token.pretty_print(printer),
                HtmlIdentSeparator::Colon(token) => token.pretty_print(printer),
                HtmlIdentSeparator::Dot(token) => token.pretty_print(printer),
            }
            segment.part.pretty_print(printer);
        }
    }
}

fn is_adjacent(end: LineColumn, start: LineColumn) -> bool {
    // In real proc-macro contexts on stable Rust, `LineColumn` may be `(0, 0)`
    // for every span, which makes this check unconditionally pass. That's the
    // best we can do without nightly span APIs; tests run against the
    // proc-macro2 fallback where locations are tracked exactly.
    end == start
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> HtmlIdent {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<HtmlIdent>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    #[test]
    fn parses_plain_ident() {
        let ident = parse("div");
        assert!(ident.rest.is_empty());
        assert_eq!(ident.to_string(), "div");
    }

    #[test]
    fn parses_dash_separated_ident() {
        assert_eq!(parse("data-foo").to_string(), "data-foo");
        assert_eq!(parse("aria-label").to_string(), "aria-label");
        assert_eq!(parse("data-foo-bar-baz").to_string(), "data-foo-bar-baz");
    }

    #[test]
    fn parses_colon_separated_ident() {
        assert_eq!(parse("xmlns:xlink").to_string(), "xmlns:xlink");
    }

    #[test]
    fn parses_dot_separated_ident() {
        assert_eq!(parse("class.active").to_string(), "class.active");
    }

    #[test]
    fn parses_numeric_segment() {
        // Alpine AJAX status-code modifiers: a segment may be a bare number.
        assert_eq!(parse("x-target.422").to_string(), "x-target.422");
    }

    #[test]
    fn parses_wildcard_status_segment() {
        // `4xx`/`5xx` lex as integer literals with an identifier suffix.
        assert_eq!(parse("x-target.4xx").to_string(), "x-target.4xx");
        assert_eq!(parse("x-target.5xx").to_string(), "x-target.5xx");
    }

    #[test]
    fn whitespace_around_numeric_segment_is_rejected() {
        assert!(
            parse_err("x-target. 422")
                .contains("whitespace is not allowed inside an HTML identifier")
        );
    }

    #[test]
    fn parses_mixed_separators() {
        assert_eq!(parse("a-b:c.d").to_string(), "a-b:c.d");
    }

    #[test]
    fn parses_rust_keywords_as_segments() {
        // `type`, `for`, etc. are valid HTML attribute names.
        assert_eq!(parse("type").to_string(), "type");
        assert_eq!(parse("data-for").to_string(), "data-for");
    }

    #[test]
    fn whitespace_around_separator_is_rejected() {
        assert!(
            parse_err("data - foo").contains("whitespace is not allowed inside an HTML identifier")
        );
        assert!(
            parse_err("data -foo").contains("whitespace is not allowed inside an HTML identifier")
        );
        assert!(
            parse_err("data- foo").contains("whitespace is not allowed inside an HTML identifier")
        );
    }

    #[test]
    fn stops_before_path_separator() {
        use syn::parse::Parser;

        let parser = |input: ParseStream| -> syn::Result<(HtmlIdent, Ident)> {
            let ident = input.parse::<HtmlIdent>()?;
            let _: Token![::] = input.parse()?;
            let tail: Ident = input.parse()?;
            Ok((ident, tail))
        };
        let (ident, tail) = parser.parse_str("foo::bar").unwrap();
        assert_eq!(ident.to_string(), "foo");
        assert_eq!(tail.to_string(), "bar");
    }

    #[test]
    fn stops_before_range_operator() {
        use syn::parse::Parser;

        let parser = |input: ParseStream| -> syn::Result<(HtmlIdent, Ident)> {
            let ident = input.parse::<HtmlIdent>()?;
            let _: Token![..] = input.parse()?;
            let tail: Ident = input.parse()?;
            Ok((ident, tail))
        };
        let (ident, tail) = parser.parse_str("foo..bar").unwrap();
        assert_eq!(ident.to_string(), "foo");
        assert_eq!(tail.to_string(), "bar");
    }
}
