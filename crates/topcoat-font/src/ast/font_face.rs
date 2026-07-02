mod display;
mod family;
mod format;
mod source;
mod style;
mod tech;
mod unicode;
mod weight;

pub use display::*;
pub use family::*;
pub use format::*;
pub use source::*;
pub use style::*;
pub use tech::*;
pub use unicode::*;
pub use weight::*;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

pub struct FontFace {
    pub family: FontFamily,
    pub src: FontSources,
    pub weight: Option<FontWeight>,
    pub style: Option<FontStyle>,
    pub display: Option<FontDisplay>,
    pub unicode_range: Option<UnicodeRanges>,
}

impl Parse for FontFace {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut family = None;
        let mut src = None;
        let mut weight = None;
        let mut style = None;
        let mut display = None;
        let mut unicode_range = None;

        while !input.is_empty() {
            if FontFamily::peek(input) {
                if family.is_some() {
                    return Err(input.error("duplicate `font-family` descriptor"));
                }
                family = Some(input.parse()?);
            } else if FontSources::peek(input) {
                if src.is_some() {
                    return Err(input.error("duplicate `src` descriptor"));
                }
                src = Some(input.parse()?);
            } else if FontWeight::peek(input) {
                if weight.is_some() {
                    return Err(input.error("duplicate `font-weight` descriptor"));
                }
                weight = Some(input.parse()?);
            } else if FontStyle::peek(input) {
                if style.is_some() {
                    return Err(input.error("duplicate `font-style` descriptor"));
                }
                style = Some(input.parse()?);
            } else if FontDisplay::peek(input) {
                if display.is_some() {
                    return Err(input.error("duplicate `font-display` descriptor"));
                }
                display = Some(input.parse()?);
            } else if UnicodeRanges::peek(input) {
                if unicode_range.is_some() {
                    return Err(input.error("duplicate `unicode-range` descriptor"));
                }
                unicode_range = Some(input.parse()?);
            } else {
                return Err(input.error(
                    "expected one of `font-family`, `src`, `font-weight`, `font-style`, \
                     `font-display`, or `unicode-range`",
                ));
            }

            if input.is_empty() {
                break;
            }
            let _: Token![;] = input.parse()?;
        }

        let family =
            family.ok_or_else(|| input.error("missing required `font-family` descriptor"))?;
        let src = src.ok_or_else(|| input.error("missing required `src` descriptor"))?;

        Ok(Self {
            family,
            src,
            weight,
            style,
            display,
            unicode_range,
        })
    }
}

impl ToTokens for FontFace {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let family = &self.family;
        let src = &self.src;

        let weight = self.weight.iter();
        let style = self.style.iter();
        let display = self.display.iter();
        let unicode_range = self.unicode_range.iter();

        quote! {
            ::topcoat::font::FontFace::new(#family, #src)
                #(.with_weight(#weight))*
                #(.with_style(#style))*
                #(.with_display(#display))*
                #(.with_unicode_range(#unicode_range))*
        }
        .to_tokens(tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> FontFace {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<FontFace>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    const MINIMAL: &str = r#"font-family: "Inter"; src: local("Inter")"#;
    const FULL: &str = r#"
        font-family: "Inter";
        src: local("Inter"), url("/inter.woff2") format("woff2");
        font-weight: 400 700;
        font-style: oblique 14deg;
        font-display: swap;
        unicode-range: U+0041-005A
    "#;

    #[test]
    fn parses_in_documented_order() {
        let face = parse(FULL);
        assert!(face.weight.is_some());
        assert!(face.style.is_some());
        assert!(face.display.is_some());
        assert!(face.unicode_range.is_some());
    }

    #[test]
    fn parses_in_any_order() {
        let face = parse(
            r#"
            unicode-range: U+0041-005A;
            font-style: italic;
            font-display: optional;
            src: url("/inter.woff2") format("woff2");
            font-weight: 700;
            font-family: "Inter"
            "#,
        );
        assert!(face.weight.is_some());
        assert!(face.style.is_some());
        assert!(face.display.is_some());
        assert!(face.unicode_range.is_some());
    }

    #[test]
    fn accepts_only_the_required_descriptors() {
        let face = parse(MINIMAL);
        assert!(face.weight.is_none());
        assert!(face.style.is_none());
        assert!(face.display.is_none());
        assert!(face.unicode_range.is_none());
    }

    #[test]
    fn accepts_a_trailing_semicolon() {
        let face = parse(r#"font-family: "Inter"; src: local("Inter");"#);
        assert!(face.weight.is_none());
    }

    #[test]
    fn rejects_a_missing_font_family() {
        assert!(parse_err(r#"src: local("Inter")"#).contains("font-family"));
    }

    #[test]
    fn rejects_a_missing_src() {
        assert!(parse_err(r#"font-family: "Inter""#).contains("src"));
    }

    #[test]
    fn rejects_a_duplicate_descriptor() {
        assert!(
            parse_err(r#"font-family: "A"; font-family: "B"; src: local("A")"#)
                .contains("duplicate")
        );
    }

    #[test]
    fn rejects_an_unknown_descriptor() {
        assert!(
            parse_err(r#"font-family: "A"; src: local("A"); font-stretch: condensed"#)
                .contains("expected one of")
        );
    }

    #[test]
    fn rejects_a_missing_separator_between_descriptors() {
        assert!(parse_err(r#"font-family: "A" src: local("A")"#).contains("`;`"));
    }
}
