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
    #[must_use]
    pub fn new(family: impl Into<Cow<'static, str>>, src: FontSources) -> Self {
        Self {
            family: family.into(),
            src,
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

/// Builds a [`FontFace`] from a CSS `@font-face`-like body.
///
/// The syntax mirrors a CSS `@font-face` rule, but swaps the stringly-typed
/// parts for the crate's enums and Rust literals. Descriptors are written as
/// `name: value;` and may appear in any order; `font-family` and `src` are
/// required, the rest are optional.
///
/// The macro expands to a `const` [`FontFace`] expression, so it can be assigned
/// to a `const` or used directly inside [`FontFaces::new`].
///
/// # Descriptors
///
/// - `font-family: "Name";` — the family name (a string literal).
/// - `src: <entry>, <entry>, ...;` — one or more sources, most preferred first.
///   Each entry is:
///   - `local("Name")` — an installed font.
///   - `url("path")` — a downloadable file, optionally followed by
///     `format(<Format>)` and/or `tech(<Tech>)`.
///   - `asset(EXPR)` — a bundled [`Asset`](topcoat_asset::Asset) handle (with
///     the `asset` feature), optionally followed by `format`/`tech`.
///
///   `format` and `tech` take [`FontFormat`](crate::FontFormat) and
///   [`FontTech`](crate::FontTech) variant names, e.g. `format(Woff2)`,
///   `tech(Variations)`.
/// - `font-weight: 400;` or `font-weight: 400 700;` — a single weight or an
///   inclusive range (space-separated, as in CSS).
/// - `font-style: normal | italic | oblique | oblique 14.0 | oblique 20.0 40.0;`
///   — oblique angles are degree numbers written as float literals (CSS's `deg`
///   suffix is dropped).
/// - `unicode-range: 0x0-0xFF, 0x131, 0x152-0x153;` — comma-separated code
///   points or `start-end` ranges, written in hex (CSS's `U+` becomes `0x`).
///
/// # Examples
///
/// ```rust
/// # use topcoat_font::{FontFace, font_face};
/// #
/// const INTER: FontFace = font_face! {
///     font-family: "Inter";
///     src: local("Inter"), url("/inter.woff2") format(Woff2);
///     font-weight: 400 700;
///     font-style: italic;
///     unicode-range: 0x0-0xFF, 0x131;
/// };
/// ```
#[macro_export]
macro_rules! font_face {
    // ----- phase 1: split the body into `;`-terminated chunks -----
    //
    // Tokens are collected one at a time into the current chunk until a `;` is
    // seen, at which point the chunk is brace-wrapped and pushed onto the list.
    // This terminates each descriptor cleanly regardless of its contents.
    (@split [$($chunk:tt)*] [$($chunks:tt)*] ; $($rest:tt)*) => {
        $crate::font_face!(@split [] [$($chunks)* { $($chunk)* }] $($rest)*)
    };
    (@split [$($chunk:tt)*] [$($chunks:tt)*] $tok:tt $($rest:tt)*) => {
        $crate::font_face!(@split [$($chunk)* $tok] [$($chunks)*] $($rest)*)
    };
    // A trailing chunk with no closing `;` is still accepted.
    (@split [$($chunk:tt)+] [$($chunks:tt)*]) => {
        $crate::font_face!(@build [] [] [] [] [] $($chunks)* { $($chunk)+ })
    };
    (@split [] [$($chunks:tt)*]) => {
        $crate::font_face!(@build [] [] [] [] [] $($chunks)*)
    };

    // ----- phase 2: route each chunk into a slot (any order) -----
    //
    // Slots, in order: [family] [src] [weight] [style] [unicode-range]. Each
    // routing arm matches its descriptor at the front of the chunk list, fills
    // its slot, and recurses on the rest.
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - family : $f:literal } $($rest:tt)*) => {
        $crate::font_face!(@build [$f] [$($src)*] [$($wt)*] [$($st)*] [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { src : $($s:tt)* } $($rest:tt)*) => {
        $crate::font_face!(@build
            [$($fam)*]
            [ $crate::FontSources::new($crate::font_face!(@srcs [] $($s)*)) ]
            [$($wt)*] [$($st)*] [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - weight : $a:literal $b:literal } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*]
            [ .with_weight($crate::FontWeightRange::from_u16($a, $b)) ]
            [$($st)*] [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - weight : $a:literal } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*]
            [ .with_weight($crate::FontWeightRange::from_u16($a, $a)) ]
            [$($st)*] [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - style : normal } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*] [$($wt)*]
            [ .with_style($crate::FontStyle::Normal) ] [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - style : italic } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*] [$($wt)*]
            [ .with_style($crate::FontStyle::Italic) ] [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - style : oblique $a:literal $b:literal } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*] [$($wt)*]
            [ .with_style($crate::FontStyle::oblique_range($a, $b)) ]
            [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - style : oblique $a:literal } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*] [$($wt)*]
            [ .with_style($crate::FontStyle::oblique_angle($a)) ]
            [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - style : oblique } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*] [$($wt)*]
            [ .with_style($crate::FontStyle::oblique()) ] [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { unicode - range : $($u:tt)* } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*] [$($wt)*] [$($st)*]
            [ .with_unicode_range($crate::UnicodeRanges::new($crate::font_face!(@uranges [] $($u)*))) ]
            $($rest)*)
    };

    // All chunks consumed: build the face. Requires family and src to be set.
    (@build [$f:expr] [$src:expr] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]) => {
        $crate::FontFace::new($f, $src) $($wt)* $($st)* $($ur)*
    };
    // Anything left over means a missing required descriptor or an unknown one.
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*] $($rest:tt)*) => {
        compile_error!(
            "font_face! requires `font-family` and `src`, and only accepts \
             `font-family`, `src`, `font-weight`, `font-style`, and `unicode-range`"
        )
    };

    // ----- `src` list -----
    (@srcs [$($acc:tt)*]) => { &[$($acc)*] };
    (@srcs [$($acc:tt)*] local($n:literal) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs [$($acc)* $crate::FontSource::local_str($n),] $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) format($f:ident) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url_str($u, Some($crate::FontFormat::$f), Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) format($f:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url_str($u, Some($crate::FontFormat::$f), None),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url_str($u, None, Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs [$($acc)* $crate::FontSource::url_str($u, None, None),] $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) format($f:ident) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url_asset($a, Some($crate::FontFormat::$f), Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) format($f:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url_asset($a, Some($crate::FontFormat::$f), None),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url_asset($a, None, Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs [$($acc)* $crate::FontSource::url_asset($a, None, None),] $($($rest)*)?)
    };

    // ----- `unicode-range` list -----
    (@uranges [$($acc:tt)*]) => { &[$($acc)*] };
    (@uranges [$($acc:tt)*] $s:literal - $e:literal $(, $($rest:tt)*)?) => {
        $crate::font_face!(@uranges [$($acc)* $crate::UnicodeRange::from_u32($s, $e),] $($($rest)*)?)
    };
    (@uranges [$($acc:tt)*] $s:literal $(, $($rest:tt)*)?) => {
        $crate::font_face!(@uranges [$($acc)* $crate::UnicodeRange::from_u32($s, $s),] $($($rest)*)?)
    };

    // ----- public entry: split, then build (must be last) -----
    ($($body:tt)*) => {
        $crate::font_face!(@split [] [] $($body)*)
    };
}
