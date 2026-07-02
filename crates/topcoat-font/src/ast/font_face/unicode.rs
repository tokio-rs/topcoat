use proc_macro2::{Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Expr, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(unicode);
    custom_keyword!(range);
    custom_keyword!(U);
}

pub struct UnicodeRanges {
    pub key: UnicodeRangesKey,
    pub colon_token: Token![:],
    pub value: UnicodeRangesValue,
}

impl Parse for UnicodeRanges {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for UnicodeRanges {
    fn peek(input: ParseStream) -> bool {
        UnicodeRangesKey::peek(input)
    }
}

impl ToTokens for UnicodeRanges {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for UnicodeRanges {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct UnicodeRangesKey {
    pub unicode_kw: kw::unicode,
    pub dash_token: Token![-],
    pub range_kw: kw::range,
}

impl Parse for UnicodeRangesKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            unicode_kw: input.parse()?,
            dash_token: input.parse()?,
            range_kw: input.parse()?,
        })
    }
}

impl ParseOption for UnicodeRangesKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::unicode) && input.peek2(Token![-]) && input.peek3(kw::range)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for UnicodeRangesKey {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.unicode_kw.span().start());
        "unicode-range".pretty_print(printer);
        printer.move_cursor(self.range_kw.span().end());
    }
}

pub enum UnicodeRangesValue {
    Expr(Box<Expr>),
    Css(Punctuated<UnicodeRange, Token![,]>),
}

impl Parse for UnicodeRangesValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(kw::U) && input.peek2(Token![+]) {
            Ok(Self::Css(Punctuated::parse_separated_nonempty(input)?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

impl ToTokens for UnicodeRangesValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Css(ranges) => quote! {
                ::topcoat::font::UnicodeRanges::new(const {
                    &[#ranges]
                })
            }
            .to_tokens(tokens),
            Self::Expr(inner) => inner.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for UnicodeRangesValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use topcoat_pretty::BreakMode;

        match self {
            Self::Css(ranges) => {
                printer.scan_begin(BreakMode::Inconsistent);
                ranges.pretty_print(printer);
                printer.scan_end();
            }
            Self::Expr(inner) => inner.pretty_print(printer),
        }
    }
}

/// A single `U+...` interval, either one code point (`U+0041`) or an inclusive
/// range (`U+0041-005A`).
pub struct UnicodeRange {
    pub u_token: kw::U,
    pub plus_token: Token![+],
    pub start: UnicodeCodePoint,
    pub end: Option<UnicodeRangeEnd>,
}

impl Parse for UnicodeRange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            u_token: input.parse()?,
            plus_token: input.parse()?,
            start: input.parse()?,
            end: UnicodeRangeEnd::parse_option(input)?,
        })
    }
}

impl ToTokens for UnicodeRange {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let start = &self.start;
        match &self.end {
            None => quote! {
                ::topcoat::font::UnicodeRange::from_u32(#start, #start)
            },
            Some(end) => quote! {
                ::topcoat::font::UnicodeRange::from_u32(#start, #end)
            },
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for UnicodeRange {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.u_token.span().start());
        "U".pretty_print(printer);
        self.plus_token.pretty_print(printer);
        self.start.pretty_print(printer);
        if let Some(end) = &self.end {
            end.pretty_print(printer);
        }
    }
}

/// The `-005A` tail of a [`UnicodeRange`] that spans more than one code point.
pub struct UnicodeRangeEnd {
    pub dash_token: Token![-],
    pub code_point: UnicodeCodePoint,
}

impl Parse for UnicodeRangeEnd {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            dash_token: input.parse()?,
            code_point: input.parse()?,
        })
    }
}

impl ParseOption for UnicodeRangeEnd {
    fn peek(input: ParseStream) -> bool {
        input.peek(Token![-])
    }
}

impl ToTokens for UnicodeRangeEnd {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.code_point.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for UnicodeRangeEnd {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.dash_token.pretty_print(printer);
        self.code_point.pretty_print(printer);
    }
}

/// A single Unicode code point written as bare hexadecimal (the `0041` in
/// `U+0041`), validated to be in `U+0000..=U+10FFFF`.
pub struct UnicodeCodePoint {
    pub value: u32,
    pub span: Span,
}

impl Parse for UnicodeCodePoint {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // After `U+`, a code point lexes either as an integer literal (when it
        // starts with a digit, e.g. `0041` or `04FF`) or as an identifier (when
        // it starts with a hex letter, e.g. `D800`). Reconstruct the original
        // text from whichever token shows up and read it as hexadecimal.
        let (text, span) = input.step(|cursor| {
            if let Some((literal, rest)) = cursor.literal() {
                Ok(((literal.to_string(), literal.span()), rest))
            } else if let Some((ident, rest)) = cursor.ident() {
                Ok(((ident.to_string(), ident.span()), rest))
            } else {
                Err(cursor.error("expected a hexadecimal Unicode code point"))
            }
        })?;

        let value = u32::from_str_radix(&text, 16).map_err(|_| {
            syn::Error::new(
                span,
                format!("`{text}` is not a hexadecimal Unicode code point"),
            )
        })?;
        if value > 0x10_FFFF {
            return Err(syn::Error::new(span, "Unicode code point exceeds U+10FFFF"));
        }

        Ok(Self { value, span })
    }
}

impl ToTokens for UnicodeCodePoint {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        Literal::u32_unsuffixed(self.value).to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for UnicodeCodePoint {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        // Preserve the written hexadecimal digits (e.g. `0041`, `D800`).
        printer.move_cursor(self.span.start());
        let source = self
            .span
            .source_text()
            .expect("cannot pretty print code point without source text");
        source.pretty_print(printer);
        printer.move_cursor(self.span.end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> UnicodeRanges {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<UnicodeRanges>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    fn css(value: &UnicodeRangesValue) -> &Punctuated<UnicodeRange, Token![,]> {
        match value {
            UnicodeRangesValue::Css(ranges) => ranges,
            UnicodeRangesValue::Expr(_) => panic!("expected a CSS unicode-range list"),
        }
    }

    #[test]
    fn parses_a_single_code_point() {
        let ranges = parse("unicode-range: U+0041");
        let ranges = css(&ranges.value);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].start.value, 0x41);
        assert!(ranges[0].end.is_none());
    }

    #[test]
    fn parses_an_inclusive_range() {
        let ranges = parse("unicode-range: U+0041-005A");
        let ranges = css(&ranges.value);
        assert_eq!(ranges[0].start.value, 0x41);
        assert_eq!(ranges[0].end.as_ref().unwrap().code_point.value, 0x5A);
    }

    #[test]
    fn parses_a_comma_separated_list() {
        let ranges = parse("unicode-range: U+0000-00FF, U+0131, U+0152-0153");
        let ranges = css(&ranges.value);
        assert_eq!(ranges.len(), 3);
        assert_eq!(ranges[1].start.value, 0x131);
    }

    #[test]
    fn parses_letter_leading_code_points() {
        // `D800` lexes as an identifier rather than a literal.
        let ranges = parse("unicode-range: U+D800-DFFF");
        let ranges = css(&ranges.value);
        assert_eq!(ranges[0].start.value, 0xD800);
        assert_eq!(ranges[0].end.as_ref().unwrap().code_point.value, 0xDFFF);
    }

    #[test]
    fn parses_the_maximum_code_point() {
        let ranges = parse("unicode-range: U+10FFFF");
        assert_eq!(css(&ranges.value)[0].start.value, 0x10_FFFF);
    }

    #[test]
    fn falls_back_to_an_expression() {
        let ranges = parse("unicode-range: (my_ranges)");
        assert!(matches!(ranges.value, UnicodeRangesValue::Expr(_)));
    }

    #[test]
    fn rejects_out_of_range_code_points() {
        assert!(parse_err("unicode-range: U+110000").contains("exceeds U+10FFFF"));
    }

    #[test]
    fn rejects_non_hexadecimal_code_points() {
        assert!(parse_err("unicode-range: U+00GG").contains("hexadecimal"));
    }
}
