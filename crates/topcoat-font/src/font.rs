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

/// Declares a [`Font`] from a family name and one or more CSS `@font-face`-like
/// blocks, and registers it for discovery.
///
/// The family name is given once and injected into every block, so the faces
/// read like a CSS stylesheet without repeating it. Each `@font-face { ... }`
/// block is a [`font_face!`](crate::font_face) body (minus its `font-family`).
///
/// Expands to a `const` [`Font`]. With the `discover` feature, it is also
/// registered so [`discover_fonts`](crate::RouterBuilderFontExt::discover_fonts)
/// finds it; without it, register the returned font manually with
/// [`font`](crate::RouterBuilderFontExt::font).
///
/// # Examples
///
/// ```rust
/// use topcoat_font::{Font, font};
///
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
#[macro_export]
macro_rules! font {
    ($family:literal, $(@font-face { $($face:tt)* })+ $(,)?) => {{
        const FONT: $crate::Font = $crate::Font::new(
            $family,
            $crate::FontFaces::new(&[
                $( $crate::font_face! { font-family: $family; $($face)* } ),+
            ]),
        );
        $crate::__register_font!(FONT);
        FONT
    }};
}
