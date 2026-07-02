use proc_macro2::{Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Expr, LitInt, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(font);
    custom_keyword!(weight);
    custom_keyword!(normal);
    custom_keyword!(bold);
}

pub struct FontWeight {
    pub key: FontWeightKey,
    pub colon_token: Token![:],
    pub value: FontWeightValue,
}

impl Parse for FontWeight {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for FontWeight {
    fn peek(input: ParseStream) -> bool {
        FontWeightKey::peek(input)
    }
}

impl ToTokens for FontWeight {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontWeight {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct FontWeightKey {
    pub font_kw: kw::font,
    pub dash_token: Token![-],
    pub weight_kw: kw::weight,
}

impl Parse for FontWeightKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            font_kw: input.parse()?,
            dash_token: input.parse()?,
            weight_kw: input.parse()?,
        })
    }
}

impl ParseOption for FontWeightKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::font) && input.peek2(Token![-]) && input.peek3(kw::weight)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontWeightKey {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.font_kw.span().start());
        "font-weight".pretty_print(printer);
        printer.move_cursor(self.weight_kw.span().end());
    }
}

pub enum FontWeightValue {
    Expr(Box<Expr>),
    Css(FontWeightRange),
}

impl Parse for FontWeightValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if FontWeightLevel::peek(input) {
            Ok(Self::Css(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

impl ToTokens for FontWeightValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Css(range) => range.to_tokens(tokens),
            Self::Expr(inner) => inner.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontWeightValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::Css(range) => range.pretty_print(printer),
            Self::Expr(inner) => inner.pretty_print(printer),
        }
    }
}

/// A `font-weight` value: a single weight (`400`, `bold`) or an inclusive range
/// carried by a variable font, written as two space-separated weights
/// (`400 700`, `normal bold`).
pub struct FontWeightRange {
    pub start: FontWeightLevel,
    pub end: Option<FontWeightLevel>,
}

impl Parse for FontWeightRange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start: FontWeightLevel = input.parse()?;
        let end = FontWeightLevel::parse_option(input)?;
        if let Some(end) = &end
            && end.value < start.value
        {
            return Err(syn::Error::new(
                end.span,
                "font weight range must not be empty",
            ));
        }
        Ok(Self { start, end })
    }
}

impl ToTokens for FontWeightRange {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let start = &self.start;
        match &self.end {
            None => quote! {
                ::topcoat::font::FontWeightRange::from_u16(#start, #start)
            },
            Some(end) => quote! {
                ::topcoat::font::FontWeightRange::from_u16(#start, #end)
            },
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontWeightRange {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.start.pretty_print(printer);
        if let Some(end) = &self.end {
            " ".pretty_print(printer);
            end.pretty_print(printer);
        }
    }
}

/// A single absolute font weight, written as a bare number validated to be in
/// `100..=900`, or one of the CSS keywords `normal` (`400`) or `bold` (`700`).
pub struct FontWeightLevel {
    pub value: u16,
    pub span: Span,
}

impl Parse for FontWeightLevel {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(kw::normal) {
            let span = input.span();
            input.parse::<kw::normal>()?;
            return Ok(Self { value: 400, span });
        }
        if input.peek(kw::bold) {
            let span = input.span();
            input.parse::<kw::bold>()?;
            return Ok(Self { value: 700, span });
        }

        let literal: LitInt = input.parse()?;
        let value: u16 = literal.base10_parse()?;
        if !(100..=900).contains(&value) {
            return Err(syn::Error::new(
                literal.span(),
                "font weight out of range 100..=900",
            ));
        }
        Ok(Self {
            value,
            span: literal.span(),
        })
    }
}

impl ParseOption for FontWeightLevel {
    fn peek(input: ParseStream) -> bool {
        input.peek(LitInt) || input.peek(kw::normal) || input.peek(kw::bold)
    }
}

impl ToTokens for FontWeightLevel {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        Literal::u16_unsuffixed(self.value).to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontWeightLevel {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        // Preserve the written form (`400`, `normal`, `bold`) rather than
        // normalizing keywords to their numeric weight.
        printer.move_cursor(self.span.start());
        let source = self
            .span
            .source_text()
            .expect("cannot pretty print font weight without source text");
        source.pretty_print(printer);
        printer.move_cursor(self.span.end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> FontWeight {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<FontWeight>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    fn css(value: &FontWeightValue) -> &FontWeightRange {
        match value {
            FontWeightValue::Css(range) => range,
            FontWeightValue::Expr(_) => panic!("expected a CSS font-weight value"),
        }
    }

    #[test]
    fn parses_a_single_weight() {
        let weight = parse("font-weight: 400");
        let range = css(&weight.value);
        assert_eq!(range.start.value, 400);
        assert!(range.end.is_none());
    }

    #[test]
    fn parses_a_weight_range() {
        let weight = parse("font-weight: 400 700");
        let range = css(&weight.value);
        assert_eq!(range.start.value, 400);
        assert_eq!(range.end.as_ref().unwrap().value, 700);
    }

    #[test]
    fn accepts_the_bounds() {
        let weight = parse("font-weight: 100 900");
        let range = css(&weight.value);
        assert_eq!(range.start.value, 100);
        assert_eq!(range.end.as_ref().unwrap().value, 900);
    }

    #[test]
    fn parses_the_normal_keyword() {
        let weight = parse("font-weight: normal");
        let range = css(&weight.value);
        assert_eq!(range.start.value, 400);
        assert!(range.end.is_none());
    }

    #[test]
    fn parses_the_bold_keyword() {
        let weight = parse("font-weight: bold");
        let range = css(&weight.value);
        assert_eq!(range.start.value, 700);
        assert!(range.end.is_none());
    }

    #[test]
    fn parses_a_keyword_range() {
        let weight = parse("font-weight: normal bold");
        let range = css(&weight.value);
        assert_eq!(range.start.value, 400);
        assert_eq!(range.end.as_ref().unwrap().value, 700);
    }

    #[test]
    fn mixes_keywords_and_numbers_in_a_range() {
        let weight = parse("font-weight: 100 bold");
        let range = css(&weight.value);
        assert_eq!(range.start.value, 100);
        assert_eq!(range.end.as_ref().unwrap().value, 700);
    }

    #[test]
    fn rejects_an_empty_keyword_range() {
        assert!(parse_err("font-weight: bold normal").contains("must not be empty"));
    }

    #[test]
    fn falls_back_to_an_expression() {
        let weight = parse("font-weight: (my_weight)");
        assert!(matches!(weight.value, FontWeightValue::Expr(_)));
    }

    #[test]
    fn rejects_weights_below_the_minimum() {
        assert!(parse_err("font-weight: 99").contains("out of range"));
    }

    #[test]
    fn rejects_weights_above_the_maximum() {
        assert!(parse_err("font-weight: 901").contains("out of range"));
        assert!(parse_err("font-weight: 1000").contains("out of range"));
    }

    #[test]
    fn rejects_an_empty_range() {
        assert!(parse_err("font-weight: 700 400").contains("must not be empty"));
    }
}
