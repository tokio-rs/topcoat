//! Font faces for building CSS `@font-face` rules.

use std::{fmt::Write, ops::Deref};

use topcoat_core::runtime::context::Cx;

use crate::{CssString, FontSources, FontStyle, FontWeightRange, UnicodeRanges};

/// A single CSS `@font-face` rule: a font family backed by one set of sources,
/// scoped to an optional weight range, style, and unicode range.
///
/// Renders as a complete `@font-face { ... }` block, with the optional
/// descriptors omitted when unset.
#[derive(Debug, Clone, PartialEq)]
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

/// An ordered, non-empty list of [`FontFace`]s.
///
/// Renders as the faces' `@font-face` rules, separated by a space.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFaces(&'static [FontFace]);

impl FontFaces {
    /// Wrap a slice of faces.
    ///
    /// # Panics
    ///
    /// Panics if `faces` is empty.
    #[must_use]
    pub const fn new(faces: &'static [FontFace]) -> Self {
        assert!(!faces.is_empty(), "font faces must not be empty");
        Self(faces)
    }

    /// Writes the faces as space-separated CSS `@font-face` rules.
    ///
    /// # Errors
    ///
    /// Returns any error produced while writing to `f`.
    pub fn fmt(&self, cx: &Cx, f: &mut dyn Write) -> std::fmt::Result {
        for (index, face) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(" ")?;
            }
            face.fmt(cx, &mut *f)?;
        }
        Ok(())
    }
}

impl Deref for FontFaces {
    type Target = [FontFace];

    fn deref(&self) -> &Self::Target {
        &self.0
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

    fn render_all(faces: &FontFaces) -> String {
        let cx = Cx::empty();
        let mut out = String::new();
        faces.fmt(&cx, &mut out).unwrap();
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

    #[test]
    fn faces_render_space_separated() {
        const FACES: FontFaces =
            FontFaces::new(&[FontFace::new("Inter", SRC), FontFace::new("Roboto", SRC)]);
        assert_eq!(
            render_all(&FACES),
            concat!(
                r#"@font-face { font-family: "Inter"; src: local("Inter") } "#,
                r#"@font-face { font-family: "Roboto"; src: local("Inter") }"#,
            ),
        );
    }

    #[test]
    fn single_face_renders_without_separator() {
        const FACES: FontFaces = FontFaces::new(&[FontFace::new("Inter", SRC)]);
        assert_eq!(
            render_all(&FACES),
            r#"@font-face { font-family: "Inter"; src: local("Inter") }"#,
        );
    }

    #[test]
    #[should_panic = "must not be empty"]
    fn new_panics_on_empty_faces() {
        let _ = FontFaces::new(&[]);
    }
}
