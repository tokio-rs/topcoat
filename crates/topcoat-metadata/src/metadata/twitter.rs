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

/// X preview settings from one metadata entry or from the combined page metadata.
#[derive(Default)]
pub(crate) struct TwitterFields {
    pub(crate) card: Field<TwitterCard>,
    pub(crate) site: Field<Cow<'static, str>>,
    pub(crate) creator: Field<Cow<'static, str>>,
}

fill_fields!(TwitterFields {
    card,
    site,
    creator,
});

/// A builder for X preview settings in [`Metadata::twitter`](crate::Metadata::twitter).
pub struct TwitterBuilder(pub(crate) TwitterFields);

impl TwitterBuilder {
    /// Chooses the format of link previews on X.
    ///
    /// Pages with [Open Graph images](crate::OpenGraphBuilder::image) default
    /// to [`SummaryLargeImage`](TwitterCard::SummaryLargeImage). Pages
    /// without images default to [`Summary`](TwitterCard::Summary).
    #[must_use]
    pub fn card(mut self, card: TwitterCard) -> Self {
        self.0.card = Field::Set(card);
        self
    }

    /// Sets the website's account on X, including `@`, for example `@acme`.
    #[must_use]
    pub fn site(mut self, site: impl Into<Cow<'static, str>>) -> Self {
        self.0.site = Field::Set(site.into());
        self
    }

    /// Sets the author's account on X, including `@`, for example `@ada`.
    #[must_use]
    pub fn creator(mut self, creator: impl Into<Cow<'static, str>>) -> Self {
        self.0.creator = Field::Set(creator.into());
        self
    }
}

/// The page's settings for link previews on X.
///
/// Call [`PageMetadata::twitter`] to read these settings. Preview titles,
/// descriptions, and images come from the [Open Graph metadata](crate::OpenGraph).
#[derive(Clone, Copy)]
pub struct Twitter<'a> {
    page: &'a PageMetadata,
}

impl<'a> Twitter<'a> {
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
