//! Font faces for building CSS `@font-face` rules.

use std::{borrow::Cow, fmt::Write, ops::Deref};

use topcoat_core::runtime::{context::Cx, fnv1a};

use crate::runtime::{CssString, FontSources, FontStyle, FontWeightRange, UnicodeRanges};

/// A single CSS `@font-face` rule: a font family backed by one set of sources,
/// scoped to an optional weight range, style, and unicode range.
///
/// Renders as a complete `@font-face { ... }` block, with the optional
/// descriptors omitted when unset.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFace {
    family: Cow<'static, str>,
    src: FontSources,
    weight: Option<FontWeightRange>,
    style: Option<FontStyle>,
    unicode_range: Option<UnicodeRanges>,
}

impl FontFace {
    /// Creates a face for `family`, served from `src`.
    ///
    /// The weight, style, and unicode range start unset; add them with
    /// [`with_weight`](Self::with_weight), [`with_style`](Self::with_style), and
    /// [`with_unicode_range`](Self::with_unicode_range). Use
    /// [`const_new`](Self::const_new) in a `const` context.
    ///
    /// # Panics
    ///
    /// Panics of the [`TryInto`] conversion of `src` fails.
    #[must_use]
    pub fn new(family: impl Into<Cow<'static, str>>, src: impl TryInto<FontSources>) -> Self {
        Self {
            family: family.into(),
            src: src
                .try_into()
                .unwrap_or_else(|_| panic!("`src` cannot be empty")),
            weight: None,
            style: None,
            unicode_range: None,
        }
    }

    /// Creates a face for `family`, served from `src`, usable in a `const` context.
    ///
    /// The weight, style, and unicode range start unset; add them with
    /// [`with_weight`](Self::with_weight), [`with_style`](Self::with_style), and
    /// [`with_unicode_range`](Self::with_unicode_range).
    #[must_use]
    pub const fn const_new(family: &'static str, src: FontSources) -> Self {
        Self {
            family: Cow::Borrowed(family),
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
        CssString(&mut *f).write_str(&self.family)?;
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

    /// Folds this face into a running content hash.
    pub(crate) const fn hash(&self, h: u64) -> u64 {
        let family_bytes = match &self.family {
            Cow::Borrowed(s) => s.as_bytes(),
            Cow::Owned(s) => s.as_bytes(),
        };
        let h = fnv1a::hash_continue(h, family_bytes);
        let h = self.src.hash(h);
        let h = match self.weight {
            Some(weight) => weight.hash(fnv1a::hash_continue(h, &[1])),
            None => fnv1a::hash_continue(h, &[0]),
        };
        let h = match self.style {
            Some(style) => style.hash(fnv1a::hash_continue(h, &[1])),
            None => fnv1a::hash_continue(h, &[0]),
        };
        match self.unicode_range {
            Some(unicode_range) => unicode_range.hash(fnv1a::hash_continue(h, &[1])),
            None => fnv1a::hash_continue(h, &[0]),
        }
    }
}

/// An ordered, non-empty list of [`FontFace`]s.
///
/// Renders as the faces' `@font-face` rules, separated by a space.
#[derive(Debug, Clone, PartialEq)]
pub struct FontFaces(Cow<'static, [FontFace]>);

impl FontFaces {
    /// Creates a list of `faces`.
    ///
    /// Use [`const_new`](Self::const_new) in a `const` context.
    ///
    /// # Panics
    ///
    /// Panics if `faces` is empty.
    #[must_use]
    pub fn new(faces: impl Into<Cow<'static, [FontFace]>>) -> Self {
        let faces = faces.into();
        assert!(!faces.is_empty(), "font faces must not be empty");
        Self(faces)
    }

    /// Creates a list from a `&'static` slice of `faces`, usable in a `const`.
    ///
    /// # Panics
    ///
    /// Panics if `faces` is empty.
    #[must_use]
    pub const fn const_new(faces: &'static [FontFace]) -> Self {
        assert!(!faces.is_empty(), "font faces must not be empty");
        Self(Cow::Borrowed(faces))
    }

    /// Folds these faces into a running content hash.
    pub(crate) const fn hash(&self, mut h: u64) -> u64 {
        let faces = match &self.0 {
            Cow::Borrowed(faces) => *faces,
            Cow::Owned(faces) => faces.as_slice(),
        };
        let mut i = 0;
        while i < faces.len() {
            h = faces[i].hash(h);
            i += 1;
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
    pub const fn as_slice(&self) -> &[FontFace] {
        match &self.0 {
            Cow::Borrowed(inner) => inner,
            Cow::Owned(inner) => inner.as_slice(),
        }
    }

    /// Builds a [`FontFaces`] from `faces`, validating the non-empty invariant.
    fn try_from_cow(faces: Cow<'static, [FontFace]>) -> Result<Self, EmptyFontFacesError> {
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

impl TryFrom<&'static [FontFace]> for FontFaces {
    type Error = EmptyFontFacesError;

    fn try_from(faces: &'static [FontFace]) -> Result<Self, Self::Error> {
        Self::try_from_cow(Cow::Borrowed(faces))
    }
}

impl TryFrom<Vec<FontFace>> for FontFaces {
    type Error = EmptyFontFacesError;

    fn try_from(faces: Vec<FontFace>) -> Result<Self, Self::Error> {
        Self::try_from_cow(Cow::Owned(faces))
    }
}

impl TryFrom<Cow<'static, [FontFace]>> for FontFaces {
    type Error = EmptyFontFacesError;

    fn try_from(faces: Cow<'static, [FontFace]>) -> Result<Self, Self::Error> {
        Self::try_from_cow(faces)
    }
}

impl Deref for FontFaces {
    type Target = [FontFace];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
