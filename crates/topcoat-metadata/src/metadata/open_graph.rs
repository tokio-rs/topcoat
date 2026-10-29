use std::sync::Arc;

use crate::{Field, Image, PageMetadata, Text, Url};

/// The kind of object a page describes, rendered as `og:type`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpenGraphType {
    /// A general page. Pages are websites unless they set another type.
    #[default]
    Website,
    /// An article, such as a blog post or a news story.
    Article,
}

impl OpenGraphType {
    /// Returns the value of `og:type` for this type.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Website => "website",
            Self::Article => "article",
        }
    }
}

/// The Open Graph fields of one level of metadata, or of a whole page once
/// combined.
#[derive(Default)]
pub(crate) struct OpenGraphFields {
    pub(crate) title: Field<Text>,
    pub(crate) description: Field<Text>,
    pub(crate) url: Field<Url>,
    pub(crate) site_name: Field<Text>,
    pub(crate) kind: Field<OpenGraphType>,
    pub(crate) images: Field<Arc<[Image]>>,
}

impl OpenGraphFields {
    /// Fills the fields that are still unset from `fields`, cloning its
    /// values.
    pub(crate) fn fill_from(&mut self, fields: &OpenGraphFields) {
        self.title.fill(fields.title.as_ref().map(Text::clone));
        self.description
            .fill(fields.description.as_ref().map(Text::clone));
        self.url.fill(fields.url.as_ref().map(Url::clone));
        self.site_name
            .fill(fields.site_name.as_ref().map(Text::clone));
        self.kind
            .fill(fields.kind.as_ref().map(OpenGraphType::clone));
        self.images.fill(fields.images.as_ref().map(Arc::clone));
    }

    /// Fills the fields that are still unset from loaded `fields`, taking
    /// its values.
    pub(crate) fn fill_from_loaded(&mut self, fields: OpenGraphFields) {
        self.title.fill(fields.title);
        self.description.fill(fields.description);
        self.url.fill(fields.url);
        self.site_name.fill(fields.site_name);
        self.kind.fill(fields.kind);
        self.images.fill(fields.images);
    }
}

/// Sets the Open Graph fields in
/// [`Metadata::open_graph`](crate::Metadata::open_graph).
pub struct OpenGraphBuilder(pub(crate) OpenGraphFields);

impl OpenGraphBuilder {
    /// Sets the title social networks show for links to the page.
    ///
    /// Defaults to the [title](crate::Metadata::title) of the page.
    #[must_use]
    pub fn title(mut self, title: impl Into<Text>) -> Self {
        self.0.title = Field::Set(title.into());
        self
    }

    /// Sets the description social networks show for links to the page.
    ///
    /// Defaults to the [description](crate::Metadata::description) of the
    /// page.
    #[must_use]
    pub fn description(mut self, description: impl Into<Text>) -> Self {
        self.0.description = Field::Set(description.into());
        self
    }

    /// Sets the URL that identifies the page on social networks.
    ///
    /// Defaults to the [canonical URL](crate::Metadata::canonical) of the
    /// page.
    #[must_use]
    pub fn url(mut self, url: impl Into<Url>) -> Self {
        self.0.url = Field::Set(url.into());
        self
    }

    /// Sets the name of the website the page belongs to.
    #[must_use]
    pub fn site_name(mut self, site_name: impl Into<Text>) -> Self {
        self.0.site_name = Field::Set(site_name.into());
        self
    }

    /// Sets the kind of object the page describes.
    #[must_use]
    pub fn kind(mut self, kind: OpenGraphType) -> Self {
        self.0.kind = Field::Set(kind);
        self
    }

    /// Adds an image that social networks may show for links to the page.
    ///
    /// The images of a level replace all images set further up.
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

    /// Removes the images set further up, so the page has none.
    #[must_use]
    pub fn no_images(mut self) -> Self {
        self.0.images = Field::Removed;
        self
    }
}

/// Reads the Open Graph metadata of a page, which social networks use for
/// link previews.
///
/// Get it with [`PageMetadata::open_graph`]. The title, description, and URL
/// fall back to the page's own when no level sets them for Open Graph.
#[derive(Clone, Copy)]
pub struct OpenGraph<'a> {
    page: &'a PageMetadata,
}

impl<'a> OpenGraph<'a> {
    /// Creates a reader for the Open Graph metadata of `page`.
    pub(crate) fn new(page: &'a PageMetadata) -> Self {
        Self { page }
    }

    /// Returns the title, or the page's title.
    #[must_use]
    pub fn title(self) -> Option<&'a Text> {
        self.page.open_graph.title.value_or(self.page.title())
    }

    /// Returns the description, or the page's description.
    #[must_use]
    pub fn description(self) -> Option<&'a Text> {
        self.page
            .open_graph
            .description
            .value_or(self.page.description())
    }

    /// Returns the URL, or the page's canonical URL.
    #[must_use]
    pub fn url(self) -> Option<&'a Url> {
        self.page.open_graph.url.value_or(self.page.canonical())
    }

    /// Returns the name of the website the page belongs to.
    #[must_use]
    pub fn site_name(self) -> Option<&'a Text> {
        self.page.open_graph.site_name.value()
    }

    /// Returns the kind of object the page describes.
    #[must_use]
    pub fn kind(self) -> OpenGraphType {
        self.page
            .open_graph
            .kind
            .value()
            .copied()
            .unwrap_or_default()
    }

    /// Returns the images social networks may show for links to the page.
    #[must_use]
    pub fn images(self) -> &'a [Image] {
        self.page
            .open_graph
            .images
            .value()
            .map_or(&[], |images| images)
    }
}
