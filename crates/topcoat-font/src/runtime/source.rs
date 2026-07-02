//! Font sources for building the `src` descriptor of a CSS `@font-face` rule.

use std::{fmt::Write, ops::Deref};

use topcoat_core::runtime::{context::Cx, fnv1a};

use crate::runtime::{CssString, FontFormat, FontTech};

/// The location of a font file, the URL of a `url()` entry in a CSS
/// `@font-face` `src` descriptor.
///
/// A [`Str`](Self::Str) is written verbatim; an [`Asset`](Self::Asset) is
/// resolved to its hosted router URL when formatted. Either is escaped as a CSS
/// `<string>` so it is safe between the quotes of `url("...")`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontSourceUrl {
    /// A URL written as-is, such as an absolute URL or an external host.
    Str(String),
    /// A bundled [`Asset`](topcoat_asset::Asset), resolved to its hosted URL.
    #[cfg(feature = "asset")]
    Asset(topcoat_asset::Asset),
}

impl FontSourceUrl {
    /// Writes the URL, escaped as the body of a CSS `<string>` (without the
    /// surrounding quotes).
    ///
    /// # Errors
    ///
    /// Returns any error produced while writing to `f`.
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

    /// Returns the inner URL if this is a [`Str`], otherwise `None`.
    ///
    /// [`Str`]: FontSourceUrl::Str
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(v) => Some(v),
            #[cfg(feature = "asset")]
            Self::Asset(_) => None,
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

    /// Returns the inner asset if this is an [`Asset`], otherwise `None`.
    ///
    /// [`Asset`]: FontSourceUrl::Asset
    #[must_use]
    #[cfg(feature = "asset")]
    pub fn as_asset(&self) -> Option<&topcoat_asset::Asset> {
        match self {
            Self::Asset(v) => Some(v),
            Self::Str(_) => None,
        }
    }

    /// Folds this URL into a running content hash.
    pub(crate) fn hash(&self, h: u64) -> u64 {
        match self {
            Self::Str(inner) => {
                fnv1a::hash_continue(fnv1a::hash_continue(h, b"s"), inner.as_bytes())
            }
            #[cfg(feature = "asset")]
            Self::Asset(inner) => {
                fnv1a::hash_continue(fnv1a::hash_continue(h, b"a"), &inner.as_u64().to_le_bytes())
            }
        }
    }
}

impl From<&str> for FontSourceUrl {
    fn from(v: &str) -> Self {
        Self::Str(v.to_owned())
    }
}

impl From<String> for FontSourceUrl {
    fn from(v: String) -> Self {
        Self::Str(v)
    }
}

#[cfg(feature = "asset")]
impl From<topcoat_asset::Asset> for FontSourceUrl {
    fn from(v: topcoat_asset::Asset) -> Self {
        Self::Asset(v)
    }
}

#[cfg(feature = "view")]
impl topcoat_view::runtime::AttributeValueViewParts for FontSourceUrl {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, parts: &mut topcoat_view::runtime::ViewParts) {
        match self {
            Self::Str(inner) => inner.into_view_parts(parts),
            #[cfg(feature = "asset")]
            Self::Asset(inner) => inner.into_view_parts(parts),
        }
    }
}

/// A single entry of a CSS `@font-face` `src` descriptor.
///
/// A [`Url`](Self::Url) points at a font file to download, with optional
/// `format()` and `tech()` hints the browser uses to skip files it cannot use.
/// A [`Local`](Self::Local) names a font already installed on the system.
///
/// Renders as the corresponding CSS, e.g. `url("/font.woff2") format(woff2)` or
/// `local("Helvetica Neue")`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontSource {
    /// A downloadable font file, with optional format and technology hints.
    Url {
        /// Where the font file is located.
        url: FontSourceUrl,
        /// The font file format, written as a `format()` hint.
        format: Option<FontFormat>,
        /// The font technology, written as a `tech()` hint.
        tech: Option<FontTech>,
    },
    /// A locally installed font, named by a `local()` entry.
    Local {
        /// The family name of the installed font.
        name: String,
    },
}

impl FontSource {
    /// A downloadable source from a URL, with optional `format()` and `tech()`
    /// hints.
    #[must_use]
    pub fn url(
        url: impl Into<FontSourceUrl>,
        format: Option<FontFormat>,
        tech: Option<FontTech>,
    ) -> Self {
        Self::Url {
            url: url.into(),
            format,
            tech,
        }
    }

    /// A source naming a font already installed on the system.
    #[must_use]
    pub fn local(name: impl Into<String>) -> Self {
        Self::Local { name: name.into() }
    }

    /// Writes this source as a single CSS `src` entry.
    ///
    /// # Errors
    ///
    /// Returns any error produced while writing to `f`.
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

    /// Folds this source into a running content hash.
    pub(crate) fn hash(&self, h: u64) -> u64 {
        match self {
            Self::Url { url, format, tech } => {
                let h = url.hash(fnv1a::hash_continue(h, b"u"));
                let h = match format {
                    Some(format) => format.hash(fnv1a::hash_continue(h, &[1])),
                    None => fnv1a::hash_continue(h, &[0]),
                };
                match tech {
                    Some(tech) => tech.hash(fnv1a::hash_continue(h, &[1])),
                    None => fnv1a::hash_continue(h, &[0]),
                }
            }
            Self::Local { name } => {
                fnv1a::hash_continue(fnv1a::hash_continue(h, b"l"), name.as_bytes())
            }
        }
    }
}

/// An ordered, non-empty list of [`FontSource`]s, the value of a CSS
/// `@font-face` `src` descriptor.
///
/// Renders as the comma-separated list CSS expects, with the browser using the
/// first source it supports. Order from most to least preferred.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FontSources(Vec<FontSource>);

impl FontSources {
    /// Creates a list of `sources`.
    ///
    /// # Panics
    ///
    /// Panics if `sources` is empty; a CSS `src` descriptor requires at least
    /// one source.
    #[must_use]
    pub fn new(sources: impl Into<Vec<FontSource>>) -> Self {
        let sources = sources.into();
        assert!(!sources.is_empty(), "font sources must not be empty");
        Self(sources)
    }

    /// Folds these sources into a running content hash.
    pub(crate) fn hash(&self, mut h: u64) -> u64 {
        for source in &self.0 {
            h = source.hash(h);
        }
        h
    }

    /// Writes the sources as a comma-separated CSS `src` descriptor value.
    ///
    /// # Errors
    ///
    /// Returns any error produced while writing to `f`.
    pub fn fmt(&self, cx: &Cx, f: &mut dyn Write) -> std::fmt::Result {
        for (index, source) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            source.fmt(cx, f)?;
        }
        Ok(())
    }

    /// Returns the sources as a slice.
    ///
    /// The slice is never empty, mirroring the non-empty invariant of
    /// [`FontSources`].
    #[must_use]
    pub fn as_slice(&self) -> &[FontSource] {
        &self.0
    }

    /// Builds a [`FontSources`] from `sources`, validating the non-empty invariant.
    fn try_from_vec(sources: Vec<FontSource>) -> Result<Self, EmptyFontSourcesError> {
        if sources.is_empty() {
            return Err(EmptyFontSourcesError);
        }
        Ok(Self(sources))
    }
}

/// Error returned when converting an empty collection into [`FontSources`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyFontSourcesError;

impl std::fmt::Display for EmptyFontSourcesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("font sources must not be empty")
    }
}

impl std::error::Error for EmptyFontSourcesError {}

impl TryFrom<Vec<FontSource>> for FontSources {
    type Error = EmptyFontSourcesError;

    fn try_from(sources: Vec<FontSource>) -> Result<Self, Self::Error> {
        Self::try_from_vec(sources)
    }
}

impl TryFrom<&[FontSource]> for FontSources {
    type Error = EmptyFontSourcesError;

    fn try_from(sources: &[FontSource]) -> Result<Self, Self::Error> {
        Self::try_from_vec(sources.to_vec())
    }
}

impl Deref for FontSources {
    type Target = [FontSource];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
