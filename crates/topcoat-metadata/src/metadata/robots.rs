use crate::field::{Field, fill_fields};

/// Instructions asking search engines whether to include a page in search
/// results and follow its links after fetching it.
///
/// Use [`Metadata::robots`](crate::Metadata::robots) to set these instructions
/// and [`PageMetadata::robots`](crate::PageMetadata::robots) to read them.
/// Each setting is inherited separately, with the closest value taking
/// priority. A layout can set defaults for a whole section. Both indexing
/// and following links are allowed by default.
#[derive(Default)]
pub struct Robots {
    index: Field<bool>,
    follow: Field<bool>,
}

impl Robots {
    /// Returns whether the page allows search engines to list it in results.
    #[must_use]
    pub fn index(&self) -> bool {
        self.index.value().copied().unwrap_or(true)
    }

    /// Returns whether the page allows search engines to follow its links.
    #[must_use]
    pub fn follow(&self) -> bool {
        self.follow.value().copied().unwrap_or(true)
    }
}

fill_fields!(Robots { index, follow });

/// A builder for search engine instructions in [`Metadata::robots`](crate::Metadata::robots).
pub struct RobotsBuilder(pub(crate) Robots);

impl RobotsBuilder {
    /// Chooses whether search engines should list the page in results.
    ///
    /// Pass `false` to add `noindex` to the robots tag.
    #[must_use]
    pub fn index(mut self, index: bool) -> Self {
        self.0.index = Field::Set(index);
        self
    }

    /// Chooses whether search engines should follow the page's links.
    ///
    /// Pass `false` to add `nofollow` to the robots tag.
    #[must_use]
    pub fn follow(mut self, follow: bool) -> Self {
        self.0.follow = Field::Set(follow);
        self
    }
}
