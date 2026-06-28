//! Font sources for building the `src` descriptor of a CSS `@font-face` rule.

use std::{fmt::Write, ops::Deref};

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
    Str(&'static str),
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
    pub fn as_str(&self) -> Option<&'static str> {
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
            Self::Str(inner) => fnv1a::hash_continue(fnv1a::hash_continue(h, b"s"), inner.as_bytes()),
            #[cfg(feature = "asset")]
            Self::Asset(inner) => {
                fnv1a::hash_continue(fnv1a::hash_continue(h, b"a"), &inner.as_u64().to_le_bytes())
            }
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
        name: &'static str,
    },
}

impl FontSource {
    /// A downloadable source from a URL string, with optional `format()` and
    /// `tech()` hints.
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

    /// A downloadable source from a bundled [`Asset`](topcoat_asset::Asset),
    /// with optional `format()` and `tech()` hints.
    ///
    /// The asset is resolved to its hosted router URL when formatted.
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

    /// A source naming a font already installed on the system.
    #[must_use]
    pub const fn local(name: &'static str) -> Self {
        Self::Local { name }
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
pub struct FontSources(&'static [FontSource]);

impl FontSources {
    /// Wrap a slice of sources.
    ///
    /// # Panics
    ///
    /// Panics if `sources` is empty; a CSS `src` descriptor requires at least
    /// one source.
    #[must_use]
    pub const fn new(sources: &'static [FontSource]) -> Self {
        assert!(!sources.is_empty(), "font sources must not be empty");
        Self(sources)
    }

    /// Folds these sources into a running content hash.
    pub(crate) const fn hash(&self, mut h: u64) -> u64 {
        let mut i = 0;
        while i < self.0.len() {
            h = self.0[i].hash(h);
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
}

impl Deref for FontSources {
    type Target = [FontSource];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(source: &FontSource) -> String {
        let cx = Cx::empty();
        let mut out = String::new();
        source.fmt(&cx, &mut out).unwrap();
        out
    }

    fn render_all(sources: &FontSources) -> String {
        let cx = Cx::empty();
        let mut out = String::new();
        sources.fmt(&cx, &mut out).unwrap();
        out
    }

    #[test]
    fn url_renders_quoted() {
        let source = FontSource::url("/font.woff2", None, None);
        assert_eq!(render(&source), r#"url("/font.woff2")"#);
    }

    #[test]
    fn url_renders_format_hint() {
        let source = FontSource::url("/font.woff2", Some(FontFormat::Woff2), None);
        assert_eq!(render(&source), r#"url("/font.woff2") format(woff2)"#);
    }

    #[test]
    fn url_renders_tech_hint() {
        let source = FontSource::url("/font.woff2", None, Some(FontTech::Variations));
        assert_eq!(render(&source), r#"url("/font.woff2") tech(variations)"#);
    }

    #[test]
    fn url_renders_format_before_tech() {
        let source = FontSource::url(
            "/font.woff2",
            Some(FontFormat::Woff2),
            Some(FontTech::ColorColrV1),
        );
        assert_eq!(
            render(&source),
            r#"url("/font.woff2") format(woff2) tech(color-colrv1)"#,
        );
    }

    #[test]
    fn url_escapes_quotes_and_backslashes() {
        let source = FontSource::url(r#"/a"b\c"#, None, None);
        assert_eq!(render(&source), r#"url("/a\"b\\c")"#);
    }

    #[test]
    fn local_renders_quoted() {
        let source = FontSource::local("Helvetica Neue");
        assert_eq!(render(&source), r#"local("Helvetica Neue")"#);
    }

    #[test]
    fn local_escapes_quotes() {
        let source = FontSource::local(r#"My "Font""#);
        assert_eq!(render(&source), r#"local("My \"Font\"")"#);
    }

    #[test]
    fn sources_render_comma_separated() {
        const SOURCES: FontSources = FontSources::new(&[
            FontSource::local("Helvetica Neue"),
            FontSource::url("/font.woff2", Some(FontFormat::Woff2), None),
        ]);
        assert_eq!(
            render_all(&SOURCES),
            r#"local("Helvetica Neue"), url("/font.woff2") format(woff2)"#,
        );
    }

    #[test]
    fn single_source_renders_without_separator() {
        const SOURCES: FontSources = FontSources::new(&[FontSource::local("Inter")]);
        assert_eq!(render_all(&SOURCES), r#"local("Inter")"#);
    }

    #[test]
    #[should_panic = "must not be empty"]
    fn new_panics_on_empty_sources() {
        let _ = FontSources::new(&[]);
    }

    #[test]
    fn predicates_reflect_the_variant() {
        let url = FontSource::url("/font.woff2", None, None);
        assert!(url.is_url());
        assert!(!url.is_local());

        let local = FontSource::local("Inter");
        assert!(local.is_local());
        assert!(!local.is_url());
    }

    #[test]
    fn str_url_accessors() {
        let url = FontSourceUrl::from("/font.woff2");
        assert!(url.is_str());
        assert_eq!(url.as_str(), Some("/font.woff2"));
    }
}
