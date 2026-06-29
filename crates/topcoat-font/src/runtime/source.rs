//! Font sources for building the `src` descriptor of a CSS `@font-face` rule.

use std::{borrow::Cow, fmt::Write, ops::Deref};

use topcoat_core::runtime::{context::Cx, fnv1a};

use crate::{CssString, FontFormat, FontTech};

/// The location of a font file, the URL of a `url()` entry in a CSS
/// `@font-face` `src` descriptor.
///
/// A [`Str`](Self::Str) is written verbatim; an [`Asset`](Self::Asset) is
/// resolved to its hosted router URL when formatted. Either is escaped as a CSS
/// `<string>` so it is safe between the quotes of `url("...")`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontSourceUrl {
    /// A URL written as-is, such as an absolute URL or an external host.
    Str(Cow<'static, str>),
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
    pub(crate) const fn hash(&self, h: u64) -> u64 {
        match self {
            Self::Str(inner) => {
                let bytes = match inner {
                    Cow::Borrowed(s) => s.as_bytes(),
                    Cow::Owned(s) => s.as_bytes(),
                };
                fnv1a::hash_continue(fnv1a::hash_continue(h, b"s"), bytes)
            }
            #[cfg(feature = "asset")]
            Self::Asset(inner) => {
                fnv1a::hash_continue(fnv1a::hash_continue(h, b"a"), &inner.as_u64().to_le_bytes())
            }
        }
    }
}

impl From<&'static str> for FontSourceUrl {
    fn from(v: &'static str) -> Self {
        Self::Str(Cow::Borrowed(v))
    }
}

impl From<String> for FontSourceUrl {
    fn from(v: String) -> Self {
        Self::Str(Cow::Owned(v))
    }
}

impl From<Cow<'static, str>> for FontSourceUrl {
    fn from(v: Cow<'static, str>) -> Self {
        Self::Str(v)
    }
}

#[cfg(feature = "asset")]
impl From<topcoat_asset::Asset> for FontSourceUrl {
    fn from(v: topcoat_asset::Asset) -> Self {
        Self::Asset(v)
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
        name: Cow<'static, str>,
    },
}

impl FontSource {
    /// A downloadable source from a URL, with optional `format()` and `tech()`
    /// hints.
    ///
    /// Use [`url_str`](Self::url_str) or [`url_asset`](Self::url_asset) in a
    /// `const` context.
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

    /// A downloadable source from a URL string, with optional `format()` and
    /// `tech()` hints, usable in a `const` context.
    #[must_use]
    pub const fn url_str(
        url: &'static str,
        format: Option<FontFormat>,
        tech: Option<FontTech>,
    ) -> Self {
        Self::Url {
            url: FontSourceUrl::Str(Cow::Borrowed(url)),
            format,
            tech,
        }
    }

    /// A downloadable source from a bundled [`Asset`](topcoat_asset::Asset),
    /// with optional `format()` and `tech()` hints, usable in a `const` context.
    ///
    /// The asset is resolved to its hosted router URL when formatted.
    #[cfg(feature = "asset")]
    #[must_use]
    pub const fn url_asset(
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

    /// A source naming a font already installed on the system.
    ///
    /// Use [`local_str`](Self::local_str) in a `const` context.
    #[must_use]
    pub fn local(name: impl Into<Cow<'static, str>>) -> Self {
        Self::Local { name: name.into() }
    }

    /// A source naming a font already installed on the system, usable in a
    /// `const` context.
    #[must_use]
    pub const fn local_str(name: &'static str) -> Self {
        Self::Local {
            name: Cow::Borrowed(name),
        }
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
    pub(crate) const fn hash(&self, h: u64) -> u64 {
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
                let bytes = match name {
                    Cow::Borrowed(s) => s.as_bytes(),
                    Cow::Owned(s) => s.as_bytes(),
                };
                fnv1a::hash_continue(fnv1a::hash_continue(h, b"l"), bytes)
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
pub struct FontSources(Cow<'static, [FontSource]>);

impl FontSources {
    /// Creates a list of `sources`.
    ///
    /// Use [`const_new`](Self::const_new) in a `const` context.
    ///
    /// # Panics
    ///
    /// Panics if `sources` is empty; a CSS `src` descriptor requires at least
    /// one source.
    #[must_use]
    pub fn new(sources: impl Into<Cow<'static, [FontSource]>>) -> Self {
        let sources = sources.into();
        assert!(!sources.is_empty(), "font sources must not be empty");
        Self(sources)
    }

    /// Creates a list from a `&'static` slice of `sources`, usable in a `const`.
    ///
    /// # Panics
    ///
    /// Panics if `sources` is empty; a CSS `src` descriptor requires at least
    /// one source.
    #[must_use]
    pub const fn const_new(sources: &'static [FontSource]) -> Self {
        assert!(!sources.is_empty(), "font sources must not be empty");
        Self(Cow::Borrowed(sources))
    }

    /// Folds these sources into a running content hash.
    pub(crate) const fn hash(&self, mut h: u64) -> u64 {
        let sources = match &self.0 {
            Cow::Borrowed(sources) => *sources,
            Cow::Owned(sources) => sources.as_slice(),
        };
        let mut i = 0;
        while i < sources.len() {
            h = sources[i].hash(h);
            i += 1;
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
    pub const fn as_slice(&self) -> &[FontSource] {
        match &self.0 {
            Cow::Borrowed(inner) => inner,
            Cow::Owned(inner) => inner.as_slice(),
        }
    }

    /// Builds a [`FontSources`] from `sources`, validating the non-empty invariant.
    fn try_from_cow(sources: Cow<'static, [FontSource]>) -> Result<Self, EmptyFontSourcesError> {
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

impl TryFrom<&'static [FontSource]> for FontSources {
    type Error = EmptyFontSourcesError;

    fn try_from(sources: &'static [FontSource]) -> Result<Self, Self::Error> {
        Self::try_from_cow(Cow::Borrowed(sources))
    }
}

impl TryFrom<Vec<FontSource>> for FontSources {
    type Error = EmptyFontSourcesError;

    fn try_from(sources: Vec<FontSource>) -> Result<Self, Self::Error> {
        Self::try_from_cow(Cow::Owned(sources))
    }
}

impl TryFrom<Cow<'static, [FontSource]>> for FontSources {
    type Error = EmptyFontSourcesError;

    fn try_from(sources: Cow<'static, [FontSource]>) -> Result<Self, Self::Error> {
        Self::try_from_cow(sources)
    }
}

impl Deref for FontSources {
    type Target = [FontSource];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
