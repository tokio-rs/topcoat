use std::borrow::Cow;

use crate::{
    PageMetadata,
    field::{Field, fill_fields},
};

/// The link preview format requested from X through `twitter:card`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TwitterCard {
    /// A preview with a small square image beside the title and description.
    Summary,
    /// A preview with a large image above the title and description.
    SummaryLargeImage,
}

impl TwitterCard {
    /// Returns the string to put in the `twitter:card` tag.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::SummaryLargeImage => "summary_large_image",
        }
    }
}

/// Settings for link previews on X.
///
/// Pass it to [`Metadata::twitter`](crate::Metadata::twitter), for example
/// `.twitter(Twitter::new().site("@acme"))`. X uses the
/// [Open Graph values](crate::OpenGraph) for the preview's title,
/// description, and image. Unset fields are inherited.
#[derive(Clone, Default)]
pub struct Twitter {
    pub(crate) card: Field<TwitterCard>,
    pub(crate) site: Field<Cow<'static, str>>,
    pub(crate) creator: Field<Cow<'static, str>>,
}

impl Twitter {
    /// Creates X preview settings that inherit every field.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Chooses the format of link previews on X.
    ///
    /// Pages with [Open Graph images](crate::OpenGraph::image) default
    /// to [`SummaryLargeImage`](TwitterCard::SummaryLargeImage). Pages
    /// without images default to [`Summary`](TwitterCard::Summary).
    #[must_use]
    pub fn card(mut self, card: TwitterCard) -> Self {
        self.card = Field::Set(card);
        self
    }

    /// Sets the website's account on X, including `@`, for example `@acme`.
    #[must_use]
    pub fn site(mut self, site: impl Into<Cow<'static, str>>) -> Self {
        self.site = Field::Set(site.into());
        self
    }

    /// Sets the author's account on X, including `@`, for example `@ada`.
    #[must_use]
    pub fn creator(mut self, creator: impl Into<Cow<'static, str>>) -> Self {
        self.creator = Field::Set(creator.into());
        self
    }
}

fill_fields!(Twitter {
    card,
    site,
    creator,
});

/// The page's settings for link previews on X.
///
/// Call [`PageMetadata::twitter`] to read these settings. Preview titles,
/// descriptions, and images come from the [Open Graph values](crate::PageOpenGraph).
#[derive(Clone, Copy)]
pub struct PageTwitter<'a> {
    page: &'a PageMetadata,
}

impl<'a> PageTwitter<'a> {
    /// Provides access to the X preview settings in `page`.
    pub(crate) fn new(page: &'a PageMetadata) -> Self {
        Self { page }
    }

    /// Returns the chosen preview format. By default, pages with Open Graph
    /// images use a large image card, and other pages use a summary card.
    #[must_use]
    pub fn card(self) -> TwitterCard {
        match self.page.twitter.card.value() {
            Some(card) => *card,
            None if self.page.open_graph().images().is_empty() => TwitterCard::Summary,
            None => TwitterCard::SummaryLargeImage,
        }
    }

    /// Returns the website's account name on X.
    #[must_use]
    pub fn site(self) -> Option<&'a str> {
        self.page.twitter.site.value().map(|site| &**site)
    }

    /// Returns the page author's account name on X.
    #[must_use]
    pub fn creator(self) -> Option<&'a str> {
        self.page.twitter.creator.value().map(|creator| &**creator)
    }
}
