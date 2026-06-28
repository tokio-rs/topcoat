use std::fmt::Display;

use topcoat_core::runtime::context::Cx;

use crate::{FontFormat, FontTech};

pub enum FontSourceUrl {
    Str(&'static str),
    #[cfg(feature = "asset")]
    Asset(topcoat_asset::Asset),
}

impl FontSourceUrl {
    pub fn fmt_cx(&self, f: &mut std::fmt::Formatter<'_>, cx: &Cx) -> std::fmt::Result {
        match self {
            Self::Str(inner) => inner.fmt(f)?,
            #[cfg(feature = "asset")]
            Self::Asset(inner) => {
                use topcoat_asset::{AssetRouteResolver, bundled_asset};
                use topcoat_core::runtime::context::app_context;

                let resolver = app_context::<AssetRouteResolver>(cx);
                let bundled_asset = bundled_asset(cx, *inner);
                resolver.resolve(bundled_asset, f);
            }
        }
        Ok(())
    }

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

    pub fn fmt_cx(&self, f: &mut std::fmt::Formatter<'_>, cx: &Cx) -> std::fmt::Result {
        match self {
            Self::Url { url, format, tech } => {
                f.write_str("url(\"")?;
                url.fmt_cx(f, cx)?;
                f.write_str("\")")?;
                if let Some(format) = format {
                    write!(f, " format({format})")?;
                }
                if let Some(tech) = tech {
                    write!(f, " tech({tech})")?;
                }
            }
            Self::Local { name } => write!(f, "local({name:?})")?,
        }
        Ok(())
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
