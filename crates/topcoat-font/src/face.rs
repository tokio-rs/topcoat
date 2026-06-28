//! Font faces for building CSS `@font-face` rules.

use std::fmt::Write;

use topcoat_core::runtime::context::Cx;

use crate::{CssString, FontSources, FontStyle, FontWeightRange, UnicodeRanges};

/// A single CSS `@font-face` rule: a font family backed by one set of sources,
/// scoped to an optional weight range, style, and unicode range.
///
/// Renders as a complete `@font-face { ... }` block, with the optional
/// descriptors omitted when unset.
pub struct FontFace {
    family: &'static str,
    src: FontSources,
    weight: Option<FontWeightRange>,
    style: Option<FontStyle>,
    unicode_range: Option<UnicodeRanges>,
}

impl FontFace {
    /// A face for `family`, served from `src`.
    ///
    /// The weight, style, and unicode range default to unset; add them with
    /// [`with_weight`](Self::with_weight), [`with_style`](Self::with_style),
    /// and [`with_unicode_range`](Self::with_unicode_range).
    #[must_use]
    pub const fn new(family: &'static str, src: FontSources) -> Self {
        Self {
            family,
            src,
            weight: None,
            style: None,
            unicode_range: None,
        }
    }

    /// Sets the `font-weight` descriptor.
    #[must_use]
    pub const fn with_weight(mut self, weight: FontWeightRange) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Sets the `font-style` descriptor.
    #[must_use]
    pub const fn with_style(mut self, style: FontStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the `unicode-range` descriptor.
    #[must_use]
    pub const fn with_unicode_range(mut self, unicode_range: UnicodeRanges) -> Self {
        self.unicode_range = Some(unicode_range);
        self
    }

    /// Writes this face as a complete CSS `@font-face` rule.
    ///
    /// # Errors
    ///
    /// Returns any error produced while writing to `f`.
    pub fn fmt(&self, cx: &Cx, f: &mut dyn Write) -> std::fmt::Result {
        f.write_str("@font-face { font-family: \"")?;
        CssString(&mut *f).write_str(self.family)?;
        f.write_str("\"; src: ")?;
        self.src.fmt(cx, &mut *f)?;
        if let Some(weight) = self.weight {
            write!(f, "; font-weight: {weight}")?;
        }
        if let Some(style) = self.style {
            write!(f, "; font-style: {style}")?;
        }
        if let Some(unicode_range) = self.unicode_range {
            write!(f, "; unicode-range: {unicode_range}")?;
        }
        f.write_str(" }")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FontSource, UnicodeRange};

    const SRC: FontSources = FontSources::new(&[FontSource::local("Inter")]);
    const UNICODE: UnicodeRanges = UnicodeRanges::new(&[UnicodeRange::from_u32(0x0, 0xFF)]);

    fn render(face: &FontFace) -> String {
        let cx = Cx::empty();
        let mut out = String::new();
        face.fmt(&cx, &mut out).unwrap();
        out
    }

    #[test]
    fn minimal_face_omits_optional_descriptors() {
        let face = FontFace::new("Inter", SRC);
        assert_eq!(
            render(&face),
            r#"@font-face { font-family: "Inter"; src: local("Inter") }"#,
        );
    }

    #[test]
    fn full_face_renders_every_descriptor() {
        let face = FontFace::new("Inter", SRC)
            .with_weight(FontWeightRange::from_u16(400, 700))
            .with_style(FontStyle::Italic)
            .with_unicode_range(UNICODE);
        assert_eq!(
            render(&face),
            concat!(
                r#"@font-face { font-family: "Inter"; src: local("Inter"); "#,
                "font-weight: 400 700; font-style: italic; unicode-range: U+0000-00FF }",
            ),
        );
    }

    #[test]
    fn family_name_is_escaped() {
        let face = FontFace::new(r#"My "Font""#, SRC);
        assert_eq!(
            render(&face),
            r#"@font-face { font-family: "My \"Font\""; src: local("Inter") }"#,
        );
    }
}
