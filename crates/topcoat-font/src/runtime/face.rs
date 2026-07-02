//! Font faces for building CSS `@font-face` rules.

use std::{fmt::Write, ops::Deref};

use topcoat_core::runtime::{context::Cx, fnv1a};

use crate::runtime::{
    CssString, FontDisplay, FontSources, FontStyle, FontWeightRange, UnicodeRanges,
};

/// A single CSS `@font-face` rule: a font family backed by one set of sources,
/// scoped to an optional weight range, style, display strategy, and unicode
/// range.
///
/// Renders as a complete `@font-face { ... }` block, with the optional
/// descriptors omitted when unset.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFace {
    family: String,
    src: FontSources,
    weight: Option<FontWeightRange>,
    style: Option<FontStyle>,
    display: Option<FontDisplay>,
    unicode_range: Option<UnicodeRanges>,
}

impl FontFace {
    /// Creates a face for `family`, served from `src`.
    ///
    /// The weight, style, display strategy, and unicode range start unset; add
    /// them with [`with_weight`](Self::with_weight),
    /// [`with_style`](Self::with_style), [`with_display`](Self::with_display),
    /// and [`with_unicode_range`](Self::with_unicode_range).
    ///
    /// # Panics
    ///
    /// Panics if the [`TryInto`] conversion of `src` fails.
    #[must_use]
    pub fn new(family: impl Into<String>, src: impl TryInto<FontSources>) -> Self {
        Self {
            family: family.into(),
            src: src
                .try_into()
                .unwrap_or_else(|_| panic!("font sources must not be empty")),
            weight: None,
            style: None,
            display: None,
            unicode_range: None,
        }
    }

    /// Sets the `font-weight` descriptor.
    #[must_use]
    pub fn with_weight(mut self, weight: FontWeightRange) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Sets the `font-style` descriptor.
    #[must_use]
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the `font-display` descriptor.
    #[must_use]
    pub fn with_display(mut self, display: FontDisplay) -> Self {
        self.display = Some(display);
        self
    }

    /// Sets the `unicode-range` descriptor.
    #[must_use]
    pub fn with_unicode_range(mut self, unicode_range: UnicodeRanges) -> Self {
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
        CssString(&mut *f).write_str(&self.family)?;
        f.write_str("\"; src: ")?;
        self.src.fmt(cx, &mut *f)?;
        if let Some(weight) = self.weight {
            write!(f, "; font-weight: {weight}")?;
        }
        if let Some(style) = self.style {
            write!(f, "; font-style: {style}")?;
        }
        if let Some(display) = self.display {
            write!(f, "; font-display: {display}")?;
        }
        if let Some(unicode_range) = self.unicode_range {
            write!(f, "; unicode-range: {unicode_range}")?;
        }
        f.write_str(" }")?;
        Ok(())
    }

    /// Folds this face into a running content hash.
    pub(crate) fn hash(&self, h: u64) -> u64 {
        let h = fnv1a::hash_continue(h, self.family.as_bytes());
        let h = self.src.hash(h);
        let h = match self.weight {
            Some(weight) => weight.hash(fnv1a::hash_continue(h, &[1])),
            None => fnv1a::hash_continue(h, &[0]),
        };
        let h = match self.style {
            Some(style) => style.hash(fnv1a::hash_continue(h, &[1])),
            None => fnv1a::hash_continue(h, &[0]),
        };
        let h = match self.display {
            Some(display) => display.hash(fnv1a::hash_continue(h, &[1])),
            None => fnv1a::hash_continue(h, &[0]),
        };
        match self.unicode_range {
            Some(unicode_range) => unicode_range.hash(fnv1a::hash_continue(h, &[1])),
            None => fnv1a::hash_continue(h, &[0]),
        }
    }

    /// The `font-family` this face defines.
    #[must_use]
    pub fn family(&self) -> &str {
        &self.family
    }

    /// The sources backing this face's `src` descriptor.
    #[must_use]
    pub fn src(&self) -> &FontSources {
        &self.src
    }

    /// The `font-weight` descriptor, if set.
    #[must_use]
    pub fn weight(&self) -> Option<FontWeightRange> {
        self.weight
    }

    /// The `font-style` descriptor, if set.
    #[must_use]
    pub fn style(&self) -> Option<FontStyle> {
        self.style
    }

    /// The `font-display` descriptor, if set.
    #[must_use]
    pub fn display(&self) -> Option<FontDisplay> {
        self.display
    }

    /// The `unicode-range` descriptor, if set.
    #[must_use]
    pub fn unicode_range(&self) -> Option<UnicodeRanges> {
        self.unicode_range
    }
}

/// An ordered, non-empty list of [`FontFace`]s.
///
/// Renders as the faces' `@font-face` rules, separated by a space.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFaces(Vec<FontFace>);

impl FontFaces {
    /// Creates a list of `faces`.
    ///
    /// # Panics
    ///
    /// Panics if `faces` is empty.
    #[must_use]
    pub fn new(faces: impl Into<Vec<FontFace>>) -> Self {
        let faces = faces.into();
        assert!(!faces.is_empty(), "font faces must not be empty");
        Self(faces)
    }

    /// Folds these faces into a running content hash.
    pub(crate) fn hash(&self, mut h: u64) -> u64 {
        for face in &self.0 {
            h = face.hash(h);
        }
        h
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

    /// Returns the faces as a slice.
    ///
    /// The slice is never empty, mirroring the non-empty invariant of
    /// [`FontFaces`].
    #[must_use]
    pub fn as_slice(&self) -> &[FontFace] {
        &self.0
    }

    /// Builds a [`FontFaces`] from `faces`, validating the non-empty invariant.
    fn try_from_vec(faces: Vec<FontFace>) -> Result<Self, EmptyFontFacesError> {
        if faces.is_empty() {
            return Err(EmptyFontFacesError);
        }
        Ok(Self(faces))
    }
}

/// Error returned when converting an empty collection into [`FontFaces`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyFontFacesError;

impl std::fmt::Display for EmptyFontFacesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("font faces must not be empty")
    }
}

impl std::error::Error for EmptyFontFacesError {}

impl TryFrom<Vec<FontFace>> for FontFaces {
    type Error = EmptyFontFacesError;

    fn try_from(faces: Vec<FontFace>) -> Result<Self, Self::Error> {
        Self::try_from_vec(faces)
    }
}

impl TryFrom<&[FontFace]> for FontFaces {
    type Error = EmptyFontFacesError;

    fn try_from(faces: &[FontFace]) -> Result<Self, Self::Error> {
        Self::try_from_vec(faces.to_vec())
    }
}

impl Deref for FontFaces {
    type Target = [FontFace];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
