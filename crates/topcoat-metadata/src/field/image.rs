use std::borrow::Cow;

#[cfg(feature = "asset")]
use topcoat_asset::Asset;

use crate::{Text, Url};

/// An image in a page's metadata, such as the preview social networks show
/// for links to the page.
///
/// Create it from its URL with [`new`](Self::new) or [`From`], then describe
/// it with [`size`](Self::size) and [`alt`](Self::alt()). With a size, social
/// networks can lay out the preview before the image loads.
#[derive(Clone)]
#[non_exhaustive]
pub struct Image {
    /// Where the image is hosted.
    pub url: Url,
    /// The width of the image in pixels.
    pub width: Option<u32>,
    /// The height of the image in pixels.
    pub height: Option<u32>,
    /// A description of the image for people who cannot see it.
    pub alt: Option<Text>,
}

impl Image {
    /// Creates an image hosted at `url`.
    #[must_use]
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            width: None,
            height: None,
            alt: None,
        }
    }

    /// Sets the width and height of the image in pixels.
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Sets a description of the image for people who cannot see it.
    #[must_use]
    pub fn alt(mut self, alt: impl Into<Text>) -> Self {
        self.alt = Some(alt.into());
        self
    }
}

impl From<Url> for Image {
    fn from(url: Url) -> Self {
        Self::new(url)
    }
}

impl From<&'static str> for Image {
    fn from(url: &'static str) -> Self {
        Self::new(url)
    }
}

impl From<String> for Image {
    fn from(url: String) -> Self {
        Self::new(url)
    }
}

impl From<Cow<'static, str>> for Image {
    fn from(url: Cow<'static, str>) -> Self {
        Self::new(url)
    }
}

#[cfg(feature = "asset")]
impl From<Asset> for Image {
    fn from(asset: Asset) -> Self {
        Self::new(asset)
    }
}
