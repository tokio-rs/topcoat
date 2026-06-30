use std::borrow::Cow;

use topcoat_core::runtime::fnv1a;

use crate::runtime::FontFaces;

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    family: Cow<'static, str>,
    faces: FontFaces,
    hash: u64,
}

impl Font {
    /// Creates a font named `family`, backed by `faces`.
    ///
    /// Use [`const_new`](Self::const_new) in a `const` context.
    ///
    /// # Panics
    ///
    /// Panics if `faces` cannot be converted into a non-empty [`FontFaces`].
    #[must_use]
    pub fn new(family: impl Into<Cow<'static, str>>, faces: impl TryInto<FontFaces>) -> Self {
        let faces = faces
            .try_into()
            .unwrap_or_else(|_| panic!("font faces must not be empty"));
        Self::from_parts(family.into(), faces)
    }

    /// Creates a font named `family`, backed by `faces`, usable in a `const` context.
    #[must_use]
    pub const fn const_new(family: &'static str, faces: FontFaces) -> Self {
        Self::from_parts(Cow::Borrowed(family), faces)
    }

    const fn from_parts(family: Cow<'static, str>, faces: FontFaces) -> Self {
        let family_bytes = match &family {
            Cow::Borrowed(s) => s.as_bytes(),
            Cow::Owned(s) => s.as_bytes(),
        };
        let h = fnv1a::hash_continue(fnv1a::hash(family_bytes), b"\0");
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
/// const INTER_FACES: &[FontFace] = &[FontFace::new(
///     "Inter",
///     FontSources::new(&[FontSource::url(
///         "/inter-400.woff2",
///         Some(FontFormat::Woff2),
///         None,
///     )]),
/// )];
/// const INTER: Font = font!("Inter", INTER_FACES);
/// ```
///
/// Unlike the CSS-like form, the family name is not injected into the faces, so
/// each [`FontFace`](crate::FontFace) must already carry the matching family.
#[macro_export]
macro_rules! font {
    ($family:expr, $(@font-face { $($face:tt)* })+ $(,)?) => {{
        const FONT: $crate::runtime::Font = $crate::runtime::Font::const_new(
            $family,
const {
            $crate::runtime::FontFaces::const_new(
        const {
                &[
                    $(const { ::topcoat::font::font_face! { font-family: $family; $($face)* } }),+
                ]
        }
            )
        }
        );
        $crate::register_font!(FONT);
        FONT
    }};
    ($family:expr, $faces:expr) => {{
        const FONT: $crate::runtime::Font = $crate::runtime::Font::new(
            $family,
            $crate::runtime::FontFaces::new($faces),
        );
        $crate::register_font!(FONT);
        FONT
    }};
}

pub use font;
