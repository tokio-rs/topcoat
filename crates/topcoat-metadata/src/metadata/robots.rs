use crate::{
    PageMetadata,
    field::{Field, fill_fields},
};

/// Instructions asking search engines whether to include a page in search
/// results and follow its links after fetching it.
///
/// Pass it to [`Metadata::robots`](crate::Metadata::robots), for example
/// `.robots(Robots::new().index(false))`. Each setting is inherited
/// separately, with the closest value taking priority. A layout can set
/// defaults for a whole section. Both indexing and following links are
/// allowed by default.
#[derive(Clone, Default)]
pub struct Robots {
    pub(crate) index: Field<bool>,
    pub(crate) follow: Field<bool>,
}

impl Robots {
    /// Creates instructions that inherit every setting.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Chooses whether search engines should list the page in results.
    ///
    /// Pass `false` to add `noindex` to the robots tag.
    #[must_use]
    pub fn index(mut self, index: bool) -> Self {
        self.index = Field::Set(index);
        self
    }

    /// Chooses whether search engines should follow the page's links.
    ///
    /// Pass `false` to add `nofollow` to the robots tag.
    #[must_use]
    pub fn follow(mut self, follow: bool) -> Self {
        self.follow = Field::Set(follow);
        self
    }
}

fill_fields!(Robots { index, follow });

/// The page's instructions for search engines.
///
/// Call [`PageMetadata::robots`] to read these instructions. Settings that
/// no metadata sets allow search engines to index the page and follow its
/// links.
#[derive(Clone, Copy)]
pub struct PageRobots<'a> {
    page: &'a PageMetadata,
}

impl<'a> PageRobots<'a> {
    /// Provides access to the search engine instructions in `page`.
    pub(crate) fn new(page: &'a PageMetadata) -> Self {
        Self { page }
    }

    /// Returns whether the page allows search engines to list it in results.
    #[must_use]
    pub fn index(self) -> bool {
        self.page.robots.index.value().copied().unwrap_or(true)
    }

    /// Returns whether the page allows search engines to follow its links.
    #[must_use]
    pub fn follow(self) -> bool {
        self.page.robots.follow.value().copied().unwrap_or(true)
    }
}
