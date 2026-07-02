//! Font file formats for the `format()` hint on a CSS `@font-face` `src`
//! descriptor.

/// A font file format, as named by the `format()` hint of a CSS `@font-face`
/// `src` descriptor.
///
/// Displays as the CSS format keyword used inside `format(...)` (`woff2`,
/// `opentype`, `embedded-opentype`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontFormat {
    /// OpenType Collection (`.otc`, `.ttc`), CSS `collection`.
    Collection,
    /// Embedded OpenType (`.eot`), CSS `embedded-opentype`.
    EmbeddedOpenType,
    /// OpenType (`.otf`, `.ttf`), CSS `opentype`.
    OpenType,
    /// SVG Font (`.svg`, `.svgz`), CSS `svg`.
    ///
    /// SVG fonts are deprecated and unsupported by most modern browsers.
    Svg,
    /// TrueType (`.ttf`), CSS `truetype`.
    TrueType,
    /// WOFF 1.0 (`.woff`), CSS `woff`.
    Woff,
    /// WOFF 2.0 (`.woff2`), CSS `woff2`.
    Woff2,
}

impl FontFormat {
    /// The CSS format keyword for this format, as written inside `format(...)`.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::Collection => "collection",
            Self::EmbeddedOpenType => "embedded-opentype",
            Self::OpenType => "opentype",
            Self::Svg => "svg",
            Self::TrueType => "truetype",
            Self::Woff => "woff",
            Self::Woff2 => "woff2",
        }
    }

    /// The format named by the given CSS `format(...)` keyword, if the keyword
    /// names a known format.
    #[must_use]
    pub fn from_keyword(keyword: &str) -> Option<Self> {
        Some(match keyword {
            "collection" => Self::Collection,
            "embedded-opentype" => Self::EmbeddedOpenType,
            "opentype" => Self::OpenType,
            "svg" => Self::Svg,
            "truetype" => Self::TrueType,
            "woff" => Self::Woff,
            "woff2" => Self::Woff2,
            _ => return None,
        })
    }

    /// The MIME type of this format, as used in a `type` attribute (e.g. on a
    /// `<link rel="preload" as="font">`).
    #[must_use]
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Collection => "font/collection",
            Self::EmbeddedOpenType => "application/vnd.ms-fontobject",
            Self::OpenType => "font/otf",
            Self::Svg => "image/svg+xml",
            Self::TrueType => "font/ttf",
            Self::Woff => "font/woff",
            Self::Woff2 => "font/woff2",
        }
    }

    /// The human-readable name of this format.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Collection => "OpenType Collection",
            Self::EmbeddedOpenType => "Embedded OpenType",
            Self::OpenType => "OpenType",
            Self::Svg => "SVG Font",
            Self::TrueType => "TrueType",
            Self::Woff => "WOFF 1.0",
            Self::Woff2 => "WOFF 2.0",
        }
    }

    /// The file extensions associated with this format, without a leading dot.
    ///
    /// Some extensions are shared across formats (`ttf` is valid for both
    /// [`OpenType`](Self::OpenType) and [`TrueType`](Self::TrueType)).
    #[must_use]
    pub const fn extensions(self) -> &'static [&'static str] {
        match self {
            Self::Collection => &["otc", "ttc"],
            Self::EmbeddedOpenType => &["eot"],
            Self::OpenType => &["otf", "ttf"],
            Self::Svg => &["svg", "svgz"],
            Self::TrueType => &["ttf"],
            Self::Woff => &["woff"],
            Self::Woff2 => &["woff2"],
        }
    }

    /// Folds this format into a running content hash.
    pub(crate) const fn hash(self, h: u64) -> u64 {
        topcoat_core::runtime::fnv1a::hash_continue(h, self.keyword().as_bytes())
    }
}

impl std::fmt::Display for FontFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.keyword())
    }
}
