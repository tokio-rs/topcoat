use crate::field::{Field, fill_fields};

/// Tells search engines what to do with a page after fetching it: whether
/// to show it in their results and whether to follow its links.
///
/// Set the directives with [`Metadata::robots`](crate::Metadata::robots),
/// and read them with [`PageMetadata::robots`](crate::PageMetadata::robots).
/// Each directive comes from the closest level that sets it, so a layout
/// can hide a whole section from search results. Both are allowed when no
/// level sets them.
#[derive(Default)]
pub struct Robots {
    index: Field<bool>,
    follow: Field<bool>,
}

impl Robots {
    /// Returns whether search engines may show the page in their results.
    #[must_use]
    pub fn index(&self) -> bool {
        self.index.value().copied().unwrap_or(true)
    }

    /// Returns whether search engines may follow the links on the page.
    #[must_use]
    pub fn follow(&self) -> bool {
        self.follow.value().copied().unwrap_or(true)
    }
}

fill_fields!(Robots { index, follow });

/// Sets the directives of [`Robots`] in [`Metadata::robots`](crate::Metadata::robots).
pub struct RobotsBuilder(pub(crate) Robots);

impl RobotsBuilder {
    /// Sets whether search engines may show the page in their results.
    ///
    /// `index(false)` renders `noindex`.
    #[must_use]
    pub fn index(mut self, index: bool) -> Self {
        self.0.index = Field::Set(index);
        self
    }

    /// Sets whether search engines may follow the links on the page.
    ///
    /// `follow(false)` renders `nofollow`.
    #[must_use]
    pub fn follow(mut self, follow: bool) -> Self {
        self.0.follow = Field::Set(follow);
        self
    }
}
