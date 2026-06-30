use std::sync::LazyLock;

use topcoat_core::runtime::fnv1a;

use crate::runtime::FontFaces;

/// The owned data backing a [`Font`]: its family name, its faces, and the
/// content hash derived from them.
#[derive(Debug, Clone, PartialEq)]
pub struct FontData {
    family: String,
    faces: FontFaces,
    hash: u64,
}

impl FontData {
    /// Creates the data for a font named `family`, backed by `faces`.
    ///
    /// # Panics
    ///
    /// Panics if `faces` cannot be converted into a non-empty [`FontFaces`].
    #[must_use]
    pub fn new(family: impl Into<String>, faces: impl TryInto<FontFaces>) -> Self {
        let family = family.into();
        let faces = faces
            .try_into()
            .unwrap_or_else(|_| panic!("font faces must not be empty"));
        let h = fnv1a::hash_continue(fnv1a::hash(family.as_bytes()), b"\0");
        let hash = faces.hash(h);
        Self {
            family,
            faces,
            hash,
        }
    }

    #[must_use]
    pub fn family(&self) -> &str {
        &self.family
    }

    #[must_use]
    pub fn faces(&self) -> &FontFaces {
        &self.faces
    }

    /// The content hash of the family name and every face setting.
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.hash
    }
}

/// A lightweight, [`Copy`] handle to a font.
///
/// It holds a reference to a lazily-initialized [`FontData`], so copying a
/// `Font` is just copying a pointer; the underlying family name, faces, and
/// hash are built once, on first access.
#[derive(Debug, Clone, Copy)]
pub struct Font(&'static LazyLock<FontData>);

impl Font {
    /// Creates a font handle backed by `data`.
    #[must_use]
    pub const fn new(data: &'static LazyLock<FontData>) -> Self {
        Self(data)
    }

    #[must_use]
    pub fn family(&self) -> &str {
        self.0.family()
    }

    #[must_use]
    pub fn faces(&self) -> &FontFaces {
        self.0.faces()
    }

    /// The content hash of the family name and every face setting.
    ///
    /// It is computed once when the font data is initialized, stable across
    /// builds for identical settings, and distinct when they differ — so it can
    /// drive a cache-busting, immutable font URL.
    #[must_use]
    pub fn hash(&self) -> u64 {
        self.0.hash()
    }
}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for Font {}

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
macro_rules! register_font {
    ($font:expr) => {
        $crate::runtime::internal::inventory::submit! { $font }
    };
}

#[doc(hidden)]
#[cfg(not(feature = "discover"))]
#[macro_export]
macro_rules! register_font {
    ($font:expr) => {};
}

pub use register_font;

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
/// # use topcoat::font::{Font, font};
/// #
/// const INTER: Font = font! {
///     "Inter",
///     @font-face {
///         src: url("/inter-400.woff2") format("woff2");
///         font-weight: 400;
///     }
///     @font-face {
///         src: url("/inter-700.woff2") format("woff2");
///         font-weight: 700;
///     }
/// };
/// ```
///
/// # Expression form
///
/// Alternatively, follow the family name with a single expression that
/// evaluates to the faces — anything convertible into [`FontFaces`], such as a
/// `Vec<FontFace>`. This uses ordinary Rust syntax instead of the CSS-like
/// blocks, which is handy when the faces are built up programmatically or shared
/// between fonts:
///
/// ```rust
/// # use topcoat::font::{Font, FontFace, FontFormat, FontSource, font};
/// #
/// fn inter_faces() -> Vec<FontFace> {
///     vec![FontFace::new(
///         "Inter",
///         vec![FontSource::url("/inter-400.woff2", Some(FontFormat::Woff2), None)],
///     )]
/// }
/// const INTER: Font = font!("Inter", inter_faces());
/// ```
///
/// Unlike the CSS-like form, the family name is not injected into the faces, so
/// each [`FontFace`](crate::FontFace) must already carry the matching family.
#[macro_export]
macro_rules! font {
    ($family:expr, $(@font-face { $($face:tt)* })+ $(,)?) => {{
        static FONT_DATA: ::std::sync::LazyLock<$crate::runtime::FontData> =
            ::std::sync::LazyLock::new(|| {
                $crate::runtime::FontData::new(
                    $family,
                    ::std::vec![
                        $(::topcoat::font::font_face! { font-family: $family; $($face)* }),+
                    ],
                )
            });
        const FONT: $crate::runtime::Font = $crate::runtime::Font::new(&FONT_DATA);
        $crate::register_font!(FONT);
        FONT
    }};
    ($family:expr, $faces:expr) => {{
        static FONT_DATA: ::std::sync::LazyLock<$crate::runtime::FontData> =
            ::std::sync::LazyLock::new(|| $crate::runtime::FontData::new($family, $faces));
        const FONT: $crate::runtime::Font = $crate::runtime::Font::new(&FONT_DATA);
        $crate::register_font!(FONT);
        FONT
    }};
}

pub use font;
