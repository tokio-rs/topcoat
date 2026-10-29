use std::borrow::Cow;

use crate::{Field, PageMetadata};

/// The layout X uses for link previews, rendered as `twitter:card`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TwitterCard {
    /// A small square image next to the title and description.
    Summary,
    /// A large image above the title and description.
    SummaryLargeImage,
}

impl TwitterCard {
    /// Returns the value of `twitter:card` for this card.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::SummaryLargeImage => "summary_large_image",
        }
    }
}

/// The X fields of one level of metadata, or of a whole page once combined.
#[derive(Default)]
pub(crate) struct TwitterFields {
    pub(crate) card: Field<TwitterCard>,
    pub(crate) site: Field<Cow<'static, str>>,
    pub(crate) creator: Field<Cow<'static, str>>,
}

impl TwitterFields {
    /// Fills the fields that are still unset from `fields`, cloning its
    /// values.
    pub(crate) fn fill_from(&mut self, fields: &TwitterFields) {
        self.card.fill(fields.card.as_ref().map(TwitterCard::clone));
        self.site.fill(fields.site.as_ref().map(Cow::clone));
        self.creator.fill(fields.creator.as_ref().map(Cow::clone));
    }

    /// Fills the fields that are still unset from loaded `fields`, taking
    /// its values.
    pub(crate) fn fill_from_loaded(&mut self, fields: TwitterFields) {
        self.card.fill(fields.card);
        self.site.fill(fields.site);
        self.creator.fill(fields.creator);
    }
}

/// Sets the X fields in [`Metadata::twitter`](crate::Metadata::twitter).
pub struct TwitterBuilder(pub(crate) TwitterFields);

impl TwitterBuilder {
    /// Sets the layout X uses for link previews.
    ///
    /// Defaults to [`SummaryLargeImage`](TwitterCard::SummaryLargeImage)
    /// when the page has [Open Graph images](crate::OpenGraphBuilder::image),
    /// and to [`Summary`](TwitterCard::Summary) otherwise.
    #[must_use]
    pub fn card(mut self, card: TwitterCard) -> Self {
        self.0.card = Field::Set(card);
        self
    }

    /// Sets the X handle of the website, such as `@acme`.
    #[must_use]
    pub fn site(mut self, site: impl Into<Cow<'static, str>>) -> Self {
        self.0.site = Field::Set(site.into());
        self
    }

    /// Sets the X handle of the page's author, such as `@ada`.
    #[must_use]
    pub fn creator(mut self, creator: impl Into<Cow<'static, str>>) -> Self {
        self.0.creator = Field::Set(creator.into());
        self
    }
}

/// Reads the X metadata of a page, which X uses for link previews.
///
/// Get it with [`PageMetadata::twitter`]. X takes the title, description,
/// and image of a preview from the [Open Graph metadata](crate::OpenGraph).
#[derive(Clone, Copy)]
pub struct Twitter<'a> {
    page: &'a PageMetadata,
}

impl<'a> Twitter<'a> {
    /// Creates a reader for the X metadata of `page`.
    pub(crate) fn new(page: &'a PageMetadata) -> Self {
        Self { page }
    }

    /// Returns the layout of link previews, or the default for the page's
    /// images.
    #[must_use]
    pub fn card(self) -> TwitterCard {
        match self.page.twitter.card.value() {
            Some(card) => *card,
            None if self.page.open_graph().images().is_empty() => TwitterCard::Summary,
            None => TwitterCard::SummaryLargeImage,
        }
    }

    /// Returns the X handle of the website.
    #[must_use]
    pub fn site(self) -> Option<&'a str> {
        self.page.twitter.site.value().map(|site| &**site)
    }

    /// Returns the X handle of the page's author.
    #[must_use]
    pub fn creator(self) -> Option<&'a str> {
        self.page.twitter.creator.value().map(|creator| &**creator)
    }
}
