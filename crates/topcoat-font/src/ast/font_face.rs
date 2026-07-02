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
    /// The `font-family` descriptor. Optional in the AST because a
    /// [`font!`](super::font::Font) `@font-face` block omits it and has the
    /// family injected by the enclosing macro; the `font_face!` macro requires
    /// it.
    pub family: Option<FontFamily>,
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

impl FontFace {
    /// Emits the `FontFace::new(...)` construction, using `family` as the face's
    /// family rather than the `font-family` descriptor stored on `self` (which
    /// may be absent).
    ///
    /// This lets [`font!`](super::font::Font) reuse a `FontFace` block while
    /// injecting the family declared once on the enclosing macro.
    pub fn to_tokens_with_family(&self, family: &impl ToTokens) -> TokenStream {
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
    }
}

impl ToTokens for FontFace {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Some(family) = &self.family else {
            syn::Error::new(
                proc_macro2::Span::call_site(),
                "missing required `font-family` descriptor",
            )
            .to_compile_error()
            .to_tokens(tokens);
            return;
        };
        self.to_tokens_with_family(family).to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontFace {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;

        // The descriptors are stored in fixed fields, so recover their written
        // order from their spans to keep the output faithful and idempotent.
        let mut descriptors: Vec<(proc_macro2::LineColumn, &dyn topcoat_pretty::PrettyPrint)> =
            vec![(self.src.key.src_kw.span().start(), &self.src)];
        if let Some(family) = &self.family {
            descriptors.push((family.key.font_kw.span().start(), family));
        }
        if let Some(weight) = &self.weight {
            descriptors.push((weight.key.font_kw.span().start(), weight));
        }
        if let Some(style) = &self.style {
            descriptors.push((style.key.font_kw.span().start(), style));
        }
        if let Some(display) = &self.display {
            descriptors.push((display.key.font_kw.span().start(), display));
        }
        if let Some(unicode_range) = &self.unicode_range {
            descriptors.push((unicode_range.key.unicode_kw.span().start(), unicode_range));
        }
        descriptors.sort_by_key(|(position, _)| (position.line, position.column));

        for (index, (_, descriptor)) in descriptors.iter().enumerate() {
            descriptor.pretty_print(printer);
            if index < descriptors.len() - 1 {
                // Separate descriptors with a semicolon and lay each out on its
                // own line, mirroring the CSS `@font-face` block form.
                ";".pretty_print(printer);
                printer.scan_same_line_trivia();
                printer.scan_force_break();
                printer.scan_trivia(true, true);
            } else {
                // A trailing semicolon is only rendered when the block breaks
                // across lines, which it always does when it has more than the
                // required two descriptors on separate lines.
                printer.scan_text(";".into(), topcoat_pretty::TextMode::Break);
                printer.advance_cursor(";");
            }
        }
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
    fn allows_a_missing_font_family() {
        // The descriptor is optional at the AST level: a `font!` `@font-face`
        // block omits it and has the family injected. The `font_face!` macro
        // enforces its presence when generating code.
        let face = parse(r#"src: local("Inter")"#);
        assert!(face.family.is_none());
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
