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
        Self { family, faces, hash }
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
