use crate::{FontFormat, FontTech};

pub enum FontSourceUrl {
    Str(&'static str),
    #[cfg(feature = "asset")]
    Asset(topcoat_asset::Asset),
}

impl FontSourceUrl {
    /// Returns `true` if the font source url is [`Str`].
    ///
    /// [`Str`]: FontSourceUrl::Str
    #[must_use]
    pub fn is_str(&self) -> bool {
        matches!(self, Self::Str(..))
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&'static str> {
        if let Self::Str(v) = self {
            Some(v)
        } else {
            None
        }
    }

    /// Returns `true` if the font source url is [`Asset`].
    ///
    /// [`Asset`]: FontSourceUrl::Asset
    #[must_use]
    pub fn is_asset(&self) -> bool {
        matches!(self, Self::Asset(..))
    }

    #[must_use]
    pub fn as_asset(&self) -> Option<&topcoat_asset::Asset> {
        if let Self::Asset(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

pub enum FontSource {
    Url {
        url: FontSourceUrl,
        format: Option<FontFormat>,
        tech: Option<FontTech>,
    },
    Local {
        name: &'static str,
    },
}

impl FontSource {
    #[must_use]
    pub const fn url(
        url: &'static str,
        format: Option<FontFormat>,
        tech: Option<FontTech>,
    ) -> Self {
        Self::Url {
            url: FontSourceUrl::Str(url),
            format,
            tech,
        }
    }

    #[cfg(feature = "asset")]
    #[must_use]
    pub const fn asset(
        url: topcoat_asset::Asset,
        format: Option<FontFormat>,
        tech: Option<FontTech>,
    ) -> Self {
        Self::Url {
            url: FontSourceUrl::Asset(url),
            format,
            tech,
        }
    }

    #[must_use]
    pub const fn local(name: &'static str) -> Self {
        Self::Local { name }
    }

    /// Returns `true` if the font source is [`Url`].
    ///
    /// [`Url`]: FontSource::Url
    #[must_use]
    pub fn is_url(&self) -> bool {
        matches!(self, Self::Url { .. })
    }

    /// Returns `true` if the font source is [`Local`].
    ///
    /// [`Local`]: FontSource::Local
    #[must_use]
    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local { .. })
    }
}
