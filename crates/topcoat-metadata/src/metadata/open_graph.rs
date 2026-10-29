use std::sync::Arc;

use crate::{
    Image, PageMetadata, Text, Url,
    field::{Field, fill_fields},
};

/// The page's content type for Open Graph's `og:type` tag.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenGraphType {
    /// A general web page. This is the default type.
    #[default]
    Website,
    /// Written content such as a blog post or news article.
    Article,
}

impl OpenGraphType {
    /// Returns the string to put in the `og:type` tag.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Website => "website",
            Self::Article => "article",
        }
    }
}

/// Open Graph values from one metadata entry or from the combined page metadata.
#[derive(Default)]
pub(crate) struct OpenGraphFields {
    pub(crate) title: Field<Text>,
    pub(crate) description: Field<Text>,
    pub(crate) url: Field<Url>,
    pub(crate) site_name: Field<Text>,
    pub(crate) kind: Field<OpenGraphType>,
    pub(crate) images: Field<Arc<[Image]>>,
}

fill_fields!(OpenGraphFields {
    title,
    description,
    url,
    site_name,
    kind,
    images,
});

/// A builder for setting fields through
/// [`Metadata::open_graph`](crate::Metadata::open_graph).
pub struct OpenGraphBuilder(pub(crate) OpenGraphFields);

impl OpenGraphBuilder {
    /// Sets the title for social link previews.
    ///
    /// Uses the page's [title](crate::Metadata::title) if no Open Graph title
    /// is set or inherited.
    #[must_use]
    pub fn title(mut self, title: impl Into<Text>) -> Self {
        self.0.title = Field::Set(title.into());
        self
    }

    /// Sets the description for social link previews.
    ///
    /// Uses the page's [description](crate::Metadata::description) if no
    /// Open Graph description is set or inherited.
    #[must_use]
    pub fn description(mut self, description: impl Into<Text>) -> Self {
        self.0.description = Field::Set(description.into());
        self
    }

    /// Sets the page URL used by social networks.
    ///
    /// Uses the page's [canonical URL](crate::Metadata::canonical) if no
    /// Open Graph URL is set or inherited.
    #[must_use]
    pub fn url(mut self, url: impl Into<Url>) -> Self {
        self.0.url = Field::Set(url.into());
        self
    }

    /// Sets the website's name for link previews.
    #[must_use]
    pub fn site_name(mut self, site_name: impl Into<Text>) -> Self {
        self.0.site_name = Field::Set(site_name.into());
        self
    }

    /// Sets the page's Open Graph content type.
    #[must_use]
    pub fn kind(mut self, kind: OpenGraphType) -> Self {
        self.0.kind = Field::Set(kind);
        self
    }

    /// Adds an image for social link previews.
    ///
    /// Repeated calls add images to this metadata entry. This list replaces
    /// the entire inherited image list.
    #[must_use]
    pub fn image(mut self, image: impl Into<Image>) -> Self {
        let mut images = match self.0.images {
            Field::Set(images) => images.to_vec(),
            Field::Inherit | Field::Removed => Vec::new(),
        };
        images.push(image.into());
        self.0.images = Field::Set(images.into());
        self
    }

    /// Clears the preview images and prevents inheriting an image list.
    #[must_use]
    pub fn no_images(mut self) -> Self {
        self.0.images = Field::Removed;
        self
    }
}

/// The page's Open Graph values for social link previews.
///
/// Call [`PageMetadata::open_graph`] to read these values. If no metadata
/// sets an Open Graph title, description, or URL, the corresponding page
/// value is used.
#[derive(Clone, Copy)]
pub struct OpenGraph<'a> {
    page: &'a PageMetadata,
}

impl<'a> OpenGraph<'a> {
    /// Provides access to the Open Graph values in `page`.
    pub(crate) fn new(page: &'a PageMetadata) -> Self {
        Self { page }
    }

    /// Returns the Open Graph title, falling back to the page title.
    #[must_use]
    pub fn title(self) -> Option<&'a Text> {
        self.page.open_graph.title.value_or(self.page.title())
    }

    /// Returns the Open Graph description, falling back to the page description.
    #[must_use]
    pub fn description(self) -> Option<&'a Text> {
        self.page
            .open_graph
            .description
            .value_or(self.page.description())
    }

    /// Returns the Open Graph URL, falling back to the canonical URL.
    #[must_use]
    pub fn url(self) -> Option<&'a Url> {
        self.page.open_graph.url.value_or(self.page.canonical())
    }

    /// Returns the website name used in link previews.
    #[must_use]
    pub fn site_name(self) -> Option<&'a Text> {
        self.page.open_graph.site_name.value()
    }

    /// Returns the Open Graph content type, defaulting to a website.
    #[must_use]
    pub fn kind(self) -> OpenGraphType {
        self.page
            .open_graph
            .kind
            .value()
            .copied()
            .unwrap_or_default()
    }

    /// Returns the page's preview images, or an empty slice if there are none.
    #[must_use]
    pub fn images(self) -> &'a [Image] {
        self.page
            .open_graph
            .images
            .value()
            .map_or(&[], |images| images)
    }
}
