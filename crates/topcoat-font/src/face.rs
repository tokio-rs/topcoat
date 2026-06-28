//! Font faces for building CSS `@font-face` rules.

use std::{fmt::Write, ops::Deref};

use topcoat_core::runtime::{context::Cx, fnv1a};

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

    /// Folds this face into a running content hash.
    pub(crate) const fn hash(&self, h: u64) -> u64 {
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

    /// Folds these faces into a running content hash.
    pub(crate) const fn hash(&self, mut h: u64) -> u64 {
        let mut i = 0;
        while i < self.0.len() {
            h = self.0[i].hash(h);
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
}

impl Deref for FontFaces {
    type Target = [FontFace];

    fn deref(&self) -> &Self::Target {
        self.0
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
/// - `font-style: normal | italic | oblique | oblique 14 | oblique 20 40;` —
///   oblique angles are bare degree numbers (CSS's `deg` suffix is dropped).
/// - `unicode-range: 0x0-0xFF, 0x131, 0x152-0x153;` — comma-separated code
///   points or `start-end` ranges, written in hex (CSS's `U+` becomes `0x`).
///
/// # Examples
///
/// ```rust
/// use topcoat_font::{FontFace, font_face};
///
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
            [ .with_style($crate::FontStyle::oblique_range($a as f32, $b as f32)) ]
            [$($ur)*] $($rest)*)
    };
    (@build [$($fam:tt)*] [$($src:tt)*] [$($wt:tt)*] [$($st:tt)*] [$($ur:tt)*]
        { font - style : oblique $a:literal } $($rest:tt)*) => {
        $crate::font_face!(@build [$($fam)*] [$($src)*] [$($wt)*]
            [ .with_style($crate::FontStyle::oblique_angle($a as f32)) ]
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
        $crate::font_face!(@srcs [$($acc)* $crate::FontSource::local($n),] $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) format($f:ident) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url($u, Some($crate::FontFormat::$f), Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) format($f:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url($u, Some($crate::FontFormat::$f), None),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::url($u, None, Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] url($u:literal) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs [$($acc)* $crate::FontSource::url($u, None, None),] $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) format($f:ident) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::asset($a, Some($crate::FontFormat::$f), Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) format($f:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::asset($a, Some($crate::FontFormat::$f), None),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) tech($t:ident) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs
            [$($acc)* $crate::FontSource::asset($a, None, Some($crate::FontTech::$t)),]
            $($($rest)*)?)
    };
    (@srcs [$($acc:tt)*] asset($a:expr) $(, $($rest:tt)*)?) => {
        $crate::font_face!(@srcs [$($acc)* $crate::FontSource::asset($a, None, None),] $($($rest)*)?)
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

    #[test]
    fn macro_builds_a_minimal_face() {
        const FACE: FontFace = crate::font_face! {
            font-family: "Inter";
            src: local("Inter");
        };
        assert_eq!(FACE, FontFace::new("Inter", SRC));
    }

    #[test]
    fn macro_matches_the_builder_for_a_full_face() {
        const FACE: FontFace = crate::font_face! {
            font-family: "Inter";
            src: url("/inter.woff2") format(Woff2) tech(Variations), local("Inter");
            font-weight: 400 700;
            font-style: italic;
            unicode-range: 0x0-0xFF, 0x131;
        };
        const EXPECTED: FontFace = FontFace::new(
            "Inter",
            FontSources::new(&[
                FontSource::url(
                    "/inter.woff2",
                    Some(crate::FontFormat::Woff2),
                    Some(crate::FontTech::Variations),
                ),
                FontSource::local("Inter"),
            ]),
        )
        .with_weight(FontWeightRange::from_u16(400, 700))
        .with_style(FontStyle::Italic)
        .with_unicode_range(UnicodeRanges::new(&[
            UnicodeRange::from_u32(0x0, 0xFF),
            UnicodeRange::from_u32(0x131, 0x131),
        ]));
        assert_eq!(FACE, EXPECTED);
    }

    #[test]
    fn macro_accepts_descriptors_in_any_order() {
        const ORDERED: FontFace = crate::font_face! {
            font-family: "Inter";
            src: local("Inter");
            font-weight: 400;
            font-style: italic;
        };
        const SHUFFLED: FontFace = crate::font_face! {
            font-style: italic;
            font-weight: 400;
            src: local("Inter");
            font-family: "Inter";
        };
        assert_eq!(ORDERED, SHUFFLED);
    }

    #[test]
    fn macro_renders_expected_css() {
        const FACE: FontFace = crate::font_face! {
            font-family: "Inter";
            src: local("Inter");
            font-weight: 400 700;
            font-style: oblique 14;
        };
        assert_eq!(
            render(&FACE),
            concat!(
                r#"@font-face { font-family: "Inter"; src: local("Inter"); "#,
                "font-weight: 400 700; font-style: oblique 14deg }",
            ),
        );
    }
}
