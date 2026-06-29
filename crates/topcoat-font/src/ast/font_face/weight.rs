use proc_macro2::Span;
use syn::{
    Expr, LitInt, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(font);
    custom_keyword!(weight);
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

pub enum FontWeightValue {
    Expr(Box<Expr>),
    Css(FontWeightRange),
}

impl Parse for FontWeightValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(LitInt) {
            Ok(Self::Css(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

/// A `font-weight` value: a single weight (`400`) or an inclusive range carried
/// by a variable font, written as two space-separated weights (`400 700`).
pub struct FontWeightRange {
    pub start: FontWeightNumber,
    pub end: Option<FontWeightNumber>,
}

impl Parse for FontWeightRange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start: FontWeightNumber = input.parse()?;
        let end = FontWeightNumber::parse_option(input)?;
        if let Some(end) = &end {
            if end.value < start.value {
                return Err(syn::Error::new(end.span, "font weight range must not be empty"));
            }
        }
        Ok(Self { start, end })
    }
}

/// A single font weight written as a bare number, validated to be in
/// `100..=900`.
pub struct FontWeightNumber {
    pub value: u16,
    pub span: Span,
}

impl Parse for FontWeightNumber {
    fn parse(input: ParseStream) -> syn::Result<Self> {
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

impl ParseOption for FontWeightNumber {
    fn peek(input: ParseStream) -> bool {
        input.peek(LitInt)
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
