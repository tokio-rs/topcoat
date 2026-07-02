use proc_macro2::{Literal, Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{
    Expr, LitFloat, LitInt, Token,
    parse::{Parse, ParseStream},
};

use topcoat_core::ast::ParseOption;

mod kw {
    use syn::custom_keyword;

    custom_keyword!(font);
    custom_keyword!(style);
    custom_keyword!(normal);
    custom_keyword!(italic);
    custom_keyword!(oblique);
}

pub struct FontStyle {
    pub key: FontStyleKey,
    pub colon_token: Token![:],
    pub value: FontStyleValue,
}

impl Parse for FontStyle {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            key: input.parse()?,
            colon_token: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl ParseOption for FontStyle {
    fn peek(input: ParseStream) -> bool {
        FontStyleKey::peek(input)
    }
}

impl ToTokens for FontStyle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.value.to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontStyle {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        self.key.pretty_print(printer);
        self.colon_token.pretty_print(printer);
        " ".pretty_print(printer);
        self.value.pretty_print(printer);
    }
}

pub struct FontStyleKey {
    pub font_kw: kw::font,
    pub dash_token: Token![-],
    pub style_kw: kw::style,
}

impl Parse for FontStyleKey {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            font_kw: input.parse()?,
            dash_token: input.parse()?,
            style_kw: input.parse()?,
        })
    }
}

impl ParseOption for FontStyleKey {
    fn peek(input: ParseStream) -> bool {
        input.peek(kw::font) && input.peek2(Token![-]) && input.peek3(kw::style)
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontStyleKey {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        printer.move_cursor(self.font_kw.span().start());
        "font-style".pretty_print(printer);
        printer.move_cursor(self.style_kw.span().end());
    }
}

pub enum FontStyleValue {
    Expr(Box<Expr>),
    Css(FontStyleKind),
}

impl Parse for FontStyleValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.peek(kw::normal) || input.peek(kw::italic) || input.peek(kw::oblique) {
            Ok(Self::Css(input.parse()?))
        } else {
            Ok(Self::Expr(input.parse()?))
        }
    }
}

impl ToTokens for FontStyleValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Css(kind) => kind.to_tokens(tokens),
            Self::Expr(inner) => inner.to_tokens(tokens),
        }
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontStyleValue {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        match self {
            Self::Css(kind) => kind.pretty_print(printer),
            Self::Expr(inner) => inner.pretty_print(printer),
        }
    }
}

/// The style axis of a font face: `normal`, `italic`, or `oblique` with an
/// optional slant angle or angle range.
pub enum FontStyleKind {
    Normal(kw::normal),
    Italic(kw::italic),
    Oblique {
        oblique_kw: kw::oblique,
        angles: Option<ObliqueAngleRange>,
    },
}

impl Parse for FontStyleKind {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::normal) {
            Ok(Self::Normal(input.parse()?))
        } else if lookahead.peek(kw::italic) {
            Ok(Self::Italic(input.parse()?))
        } else if lookahead.peek(kw::oblique) {
            Ok(Self::Oblique {
                oblique_kw: input.parse()?,
                angles: ObliqueAngleRange::parse_option(input)?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for FontStyleKind {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Normal(_) => quote! { ::topcoat::font::FontStyle::Normal },
            Self::Italic(_) => quote! { ::topcoat::font::FontStyle::Italic },
            Self::Oblique { angles, .. } => match angles {
                None => quote! { ::topcoat::font::FontStyle::oblique() },
                Some(range) => range.to_token_stream(),
            },
        }
        .to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for FontStyleKind {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        use syn::spanned::Spanned;
        match self {
            Self::Normal(normal_kw) => {
                printer.move_cursor(normal_kw.span().start());
                "normal".pretty_print(printer);
                printer.move_cursor(normal_kw.span().end());
            }
            Self::Italic(italic_kw) => {
                printer.move_cursor(italic_kw.span().start());
                "italic".pretty_print(printer);
                printer.move_cursor(italic_kw.span().end());
            }
            Self::Oblique { oblique_kw, angles } => {
                printer.move_cursor(oblique_kw.span().start());
                "oblique".pretty_print(printer);
                printer.move_cursor(oblique_kw.span().end());
                if let Some(range) = angles {
                    " ".pretty_print(printer);
                    range.start.pretty_print(printer);
                    if let Some(end) = &range.end {
                        " ".pretty_print(printer);
                        end.pretty_print(printer);
                    }
                }
            }
        }
    }
}

/// The angle following `oblique`: a single angle (`14deg`) or an inclusive
/// range carried by a variable font, written as two space-separated angles
/// (`20deg 40deg`).
pub struct ObliqueAngleRange {
    pub start: ObliqueAngle,
    pub end: Option<ObliqueAngle>,
}

impl Parse for ObliqueAngleRange {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let start: ObliqueAngle = input.parse()?;
        let end = ObliqueAngle::parse_option(input)?;
        if let Some(end) = &end
            && end.degrees < start.degrees
        {
            return Err(syn::Error::new(
                end.span,
                "oblique angle range must not be empty",
            ));
        }
        Ok(Self { start, end })
    }
}

impl ParseOption for ObliqueAngleRange {
    fn peek(input: ParseStream) -> bool {
        ObliqueAngle::peek(input)
    }
}

impl ToTokens for ObliqueAngleRange {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let start = &self.start;
        match &self.end {
            None => quote! { ::topcoat::font::FontStyle::oblique_angle(#start) },
            Some(end) => quote! { ::topcoat::font::FontStyle::oblique_range(#start, #end) },
        }
        .to_tokens(tokens);
    }
}

/// A single oblique slant angle in degrees (`14deg`, `-12.5deg`), validated to
/// be in `-90deg..=90deg`.
pub struct ObliqueAngle {
    pub minus_token: Option<Token![-]>,
    pub degrees: f32,
    pub span: Span,
}

impl Parse for ObliqueAngle {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let minus_token: Option<Token![-]> = if input.peek(Token![-]) {
            Some(input.parse()?)
        } else {
            None
        };

        // An angle such as `14deg` lexes as a numeric literal with a `deg`
        // suffix (`12.5deg` as a float literal); reconstruct its text and strip
        // the unit.
        let (text, span) = input.step(|cursor| {
            cursor
                .literal()
                .map(|(literal, rest)| ((literal.to_string(), literal.span()), rest))
                .ok_or_else(|| cursor.error("expected an angle in degrees, e.g. `14deg`"))
        })?;

        let digits = text
            .strip_suffix("deg")
            .ok_or_else(|| syn::Error::new(span, "expected an angle in degrees, e.g. `14deg`"))?;
        let magnitude: f32 = digits
            .parse()
            .map_err(|_| syn::Error::new(span, format!("`{text}` is not a valid angle")))?;
        let degrees = if minus_token.is_some() {
            -magnitude
        } else {
            magnitude
        };
        if !(-90.0..=90.0).contains(&degrees) {
            return Err(syn::Error::new(
                span,
                "oblique angle out of range -90deg..=90deg",
            ));
        }

        Ok(Self {
            minus_token,
            degrees,
            span,
        })
    }
}

impl ParseOption for ObliqueAngle {
    fn peek(input: ParseStream) -> bool {
        input.peek(LitInt)
            || input.peek(LitFloat)
            || (input.peek(Token![-]) && (input.peek2(LitInt) || input.peek2(LitFloat)))
    }
}

impl ToTokens for ObliqueAngle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        Literal::f32_suffixed(self.degrees).to_tokens(tokens);
    }
}

#[cfg(feature = "pretty")]
impl topcoat_pretty::PrettyPrint for ObliqueAngle {
    fn pretty_print(&self, printer: &mut topcoat_pretty::Printer<'_>) {
        if let Some(minus_token) = &self.minus_token {
            minus_token.pretty_print(printer);
        }
        printer.move_cursor(self.span.start());
        let source = self
            .span
            .source_text()
            .expect("cannot pretty print oblique angle without source text");
        source.pretty_print(printer);
        printer.move_cursor(self.span.end());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> FontStyle {
        syn::parse_str(source).unwrap()
    }

    fn parse_err(source: &str) -> String {
        match syn::parse_str::<FontStyle>(source) {
            Ok(_) => panic!("expected parse error for `{source}`"),
            Err(err) => err.to_string(),
        }
    }

    fn css(value: &FontStyleValue) -> &FontStyleKind {
        match value {
            FontStyleValue::Css(kind) => kind,
            FontStyleValue::Expr(_) => panic!("expected a CSS font-style value"),
        }
    }

    fn oblique(kind: &FontStyleKind) -> Option<&ObliqueAngleRange> {
        match kind {
            FontStyleKind::Oblique { angles, .. } => angles.as_ref(),
            _ => panic!("expected an oblique font-style"),
        }
    }

    #[test]
    fn parses_normal_and_italic() {
        assert!(matches!(
            css(&parse("font-style: normal").value),
            FontStyleKind::Normal(_)
        ));
        assert!(matches!(
            css(&parse("font-style: italic").value),
            FontStyleKind::Italic(_)
        ));
    }

    #[test]
    fn parses_bare_oblique() {
        let style = parse("font-style: oblique");
        assert!(oblique(css(&style.value)).is_none());
    }

    #[test]
    fn parses_oblique_with_an_angle() {
        let style = parse("font-style: oblique 14deg");
        let range = oblique(css(&style.value)).unwrap();
        assert!((range.start.degrees - 14.0).abs() < 1e-6);
        assert!(range.end.is_none());
    }

    #[test]
    fn parses_a_negative_angle() {
        let style = parse("font-style: oblique -12.5deg");
        let range = oblique(css(&style.value)).unwrap();
        assert!((range.start.degrees + 12.5).abs() < 1e-6);
        assert!(range.start.minus_token.is_some());
    }

    #[test]
    fn parses_an_oblique_range() {
        let style = parse("font-style: oblique 20deg 40deg");
        let range = oblique(css(&style.value)).unwrap();
        assert!((range.start.degrees - 20.0).abs() < 1e-6);
        assert!((range.end.as_ref().unwrap().degrees - 40.0).abs() < 1e-6);
    }

    #[test]
    fn falls_back_to_an_expression() {
        let style = parse("font-style: (my_style)");
        assert!(matches!(style.value, FontStyleValue::Expr(_)));
    }

    #[test]
    fn rejects_out_of_range_angles() {
        assert!(parse_err("font-style: oblique 91deg").contains("out of range"));
        assert!(parse_err("font-style: oblique -90.1deg").contains("out of range"));
    }

    #[test]
    fn rejects_an_empty_range() {
        assert!(parse_err("font-style: oblique 40deg 20deg").contains("must not be empty"));
    }

    #[test]
    fn rejects_angles_without_a_unit() {
        assert!(parse_err("font-style: oblique 14").contains("degrees"));
    }
}
