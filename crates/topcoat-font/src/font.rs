use topcoat_core::runtime::fnv1a;

use crate::FontFaces;

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    family: &'static str,
    faces: FontFaces,
    hash: u64,
}

impl Font {
    #[must_use]
    pub const fn new(family: &'static str, faces: FontFaces) -> Self {
        let h = fnv1a::hash_continue(fnv1a::hash(family.as_bytes()), b"\0");
        let hash = faces.hash(h);
        Self {
            family,
            faces,
            hash,
        }
    }

    #[must_use]
    pub fn family(&self) -> &'static str {
        self.family
    }

    #[must_use]
    pub fn faces(&self) -> &FontFaces {
        &self.faces
    }

    /// The content hash of the family name and every face setting.
    ///
    /// It is computed once when the font is constructed, stable across builds
    /// for identical settings, and distinct when they differ — so it can drive
    /// a cache-busting, immutable font URL.
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Font);

/// Registers a [`Font`] for discovery when the `discover` feature is enabled,
/// and expands to nothing otherwise.
///
/// The feature gate lives here, in the defining crate, so it reflects
/// topcoat-font's own `discover` feature rather than the calling crate's.
#[doc(hidden)]
#[cfg(feature = "discover")]
#[macro_export]
macro_rules! __register_font {
    ($font:expr) => {
        $crate::internal::inventory::submit! { $font }
    };
}

#[doc(hidden)]
#[cfg(not(feature = "discover"))]
#[macro_export]
macro_rules! __register_font {
    ($font:expr) => {};
}

/// Declares a [`Font`] from a family name and its faces, and registers it for
/// discovery.
///
/// Expands to a `const` [`Font`]. With the `discover` feature, it is also
/// registered so [`discover_fonts`](crate::RouterBuilderFontExt::discover_fonts)
/// finds it; without it, register the returned font manually with
/// [`font`](crate::RouterBuilderFontExt::font).
///
/// The faces can be given in one of two forms.
///
/// # CSS-like form
///
/// Follow the family name with one or more CSS `@font-face`-like blocks. The
/// family name is given once and injected into every block, so the faces read
/// like a CSS stylesheet without repeating it. Each `@font-face { ... }` block
/// is a [`font_face!`](crate::font_face) body (minus its `font-family`).
///
/// ```rust
/// # use topcoat_font::{Font, font};
/// #
/// const INTER: Font = font! {
///     "Inter",
///     @font-face {
///         src: url("/inter-400.woff2") format(Woff2);
///         font-weight: 400;
///     }
///     @font-face {
///         src: url("/inter-700.woff2") format(Woff2);
///         font-weight: 700;
///     }
/// };
/// ```
///
/// # Expression form
///
/// Alternatively, follow the family name with a single expression that
/// evaluates to a `&'static [FontFace]`. This uses ordinary Rust syntax instead
/// of the CSS-like blocks, which is handy when the faces are built up
/// programmatically or shared between fonts:
///
/// ```rust
/// # use topcoat_font::{Font, FontFace, FontFormat, FontSource, FontSources, font};
/// #
/// const INTER_FACES: &[FontFace] = &[
///     FontFace::new(
///         "Inter",
///         FontSources::new(&[FontSource::url(
///             "/inter-400.woff2",
///             Some(FontFormat::Woff2),
///             None,
///         )]),
///     ),
/// ];
/// const INTER: Font = font!("Inter", INTER_FACES);
/// ```
///
/// Unlike the CSS-like form, the family name is not injected into the faces, so
/// each [`FontFace`](crate::FontFace) must already carry the matching family.
#[macro_export]
macro_rules! font {
    ($family:expr, $(@font-face { $($face:tt)* })+ $(,)?) => {{
        const FONT: $crate::Font = $crate::Font::new(
            $family,
            $crate::FontFaces::new(&[
                $( $crate::font_face! { font-family: $family; $($face)* } ),+
            ]),
        );
        $crate::__register_font!(FONT);
        FONT
    }};
    ($family:expr, $faces:expr) => {{
        const FONT: $crate::Font = $crate::Font::new(
            $family,
            $crate::FontFaces::new($faces),
        );
        $crate::__register_font!(FONT);
        FONT
    }};
}
