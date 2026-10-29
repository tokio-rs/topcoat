use std::borrow::Cow;

#[cfg(feature = "asset")]
use topcoat_asset::Asset;

use crate::{Text, Url};

/// A metadata image that can appear in a social link preview.
///
/// Pass its URL to [`new`](Self::new) or convert it with [`From`]. Add its
/// dimensions with [`size`](Self::size) and a text description with
/// [`alt`](Self::alt()). Dimensions help social networks reserve space
/// before the image loads.
#[derive(Clone)]
#[non_exhaustive]
pub struct Image {
    /// The URL used to load the image.
    pub url: Url,
    /// The image's width in pixels, if known.
    pub width: Option<u32>,
    /// The image's height in pixels, if known.
    pub height: Option<u32>,
    /// Text that describes the image for people who cannot see it.
    pub alt: Option<Text>,
}

impl Image {
    /// Creates metadata for the image at `url`.
    #[must_use]
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            width: None,
            height: None,
            alt: None,
        }
    }

    /// Records the image's dimensions in pixels.
    #[must_use]
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Adds text that describes the image for people who cannot see it.
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
