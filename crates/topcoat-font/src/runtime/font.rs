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
///
/// See the `font!` macro on how to construct a [`Font`] handle.
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
