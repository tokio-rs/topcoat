use std::{fmt::Write, ops::Deref};

use topcoat_core::runtime::context::Cx;

use crate::{CssString, FontFormat, FontTech};

pub enum FontSourceUrl {
    Str(&'static str),
    #[cfg(feature = "asset")]
    Asset(topcoat_asset::Asset),
}

impl FontSourceUrl {
    /// Writes the URL, escaped as the body of a CSS `<string>` (without the
    /// surrounding quotes).
    #[cfg_attr(not(feature = "asset"), expect(unused_variables))]
    pub fn fmt(&self, cx: &Cx, f: &mut dyn Write) -> std::fmt::Result {
        let mut f = CssString(f);
        match self {
            Self::Str(inner) => f.write_str(inner),
            #[cfg(feature = "asset")]
            Self::Asset(inner) => {
                use topcoat_asset::{AssetRouteResolver, bundled_asset};
                use topcoat_core::runtime::context::app_context;

                let resolver = app_context::<AssetRouteResolver>(cx);
                let bundled_asset = bundled_asset(cx, *inner);
                resolver.resolve(bundled_asset, &mut f)
            }
        }
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
    #[cfg(feature = "asset")]
    pub fn is_asset(&self) -> bool {
        matches!(self, Self::Asset(..))
    }

    #[must_use]
    #[cfg(feature = "asset")]
    pub fn as_asset(&self) -> Option<&topcoat_asset::Asset> {
        if let Self::Asset(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

impl From<&'static str> for FontSourceUrl {
    fn from(v: &'static str) -> Self {
        Self::Str(v)
    }
}

#[cfg(feature = "asset")]
impl From<topcoat_asset::Asset> for FontSourceUrl {
    fn from(v: topcoat_asset::Asset) -> Self {
        Self::Asset(v)
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

    pub fn fmt(&self, cx: &Cx, f: &mut dyn Write) -> std::fmt::Result {
        match self {
            Self::Url { url, format, tech } => {
                f.write_str("url(\"")?;
                url.fmt(cx, &mut *f)?;
                f.write_str("\")")?;
                if let Some(format) = format {
                    write!(f, " format({format})")?;
                }
                if let Some(tech) = tech {
                    write!(f, " tech({tech})")?;
                }
            }
            Self::Local { name } => {
                f.write_str("local(\"")?;
                CssString(f).write_str(name)?;
                f.write_str("\")")?;
            }
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

pub struct FontSources(&'static [FontSource]);

impl FontSources {
    #[must_use]
    pub const fn new(sources: &'static [FontSource]) -> Self {
        assert!(!sources.is_empty(), "font sources must not be empty");
        Self(sources)
    }

    pub fn fmt(&self, cx: &Cx, f: &mut dyn Write) -> std::fmt::Result {
        for (index, source) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            source.fmt(cx, f)?;
        }
        Ok(())
    }
}

impl Deref for FontSources {
    type Target = [FontSource];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
