use crate::FontFaces;

#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    family: &'static str,
    faces: FontFaces,
}

impl Font {
    #[must_use]
    pub const fn new(family: &'static str, faces: FontFaces) -> Self {
        Self { family, faces }
    }

    #[must_use]
    pub fn family(&self) -> &'static str {
        self.family
    }

    #[must_use]
    pub fn faces(&self) -> &FontFaces {
        &self.faces
    }
}

#[cfg(feature = "discover")]
inventory::collect!(Font);
