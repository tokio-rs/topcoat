//! Font technologies for the `tech()` hint on a CSS `@font-face` `src`
//! descriptor.

/// A font technology, as named by the `tech()` hint of a CSS `@font-face`
/// `src` descriptor.
///
/// Displays as the CSS technology keyword used inside `tech(...)`
/// (`color-colrv1`, `features-opentype`, `variations`, ...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontTech {
    /// Color bitmap data tables, CSS `color-cbdt`.
    ColorCbdt,
    /// Multi-colored glyphs via COLR version 0 table, CSS `color-colrv0`.
    ColorColrV0,
    /// Multi-colored glyphs via COLR version 1 table, CSS `color-colrv1`.
    ColorColrV1,
    /// Standard bitmap graphics tables, CSS `color-sbix`.
    ColorSbix,
    /// SVG multi-colored tables, CSS `color-svg`.
    ColorSvg,
    /// TrueType `morx` and `kerx` tables, CSS `features-aat`.
    FeaturesAat,
    /// Graphite features, namely `Silf`, `Glat`, `Gloc`, `Feat`, and `Sill`
    /// tables, CSS `features-graphite`.
    FeaturesGraphite,
    /// OpenType `GSUB` and `GPOS` tables, CSS `features-opentype`.
    FeaturesOpenType,
    /// Incremental font loading, CSS `incremental`.
    Incremental,
    /// Font palettes by means of `font-palette` to select one of many color
    /// palettes in the font, CSS `palettes`.
    Palettes,
    /// Font variations in TrueType and OpenType fonts to control the font axis,
    /// weight, glyphs, etc., CSS `variations`.
    Variations,
}

impl FontTech {
    /// The CSS technology keyword for this technology, as written inside
    /// `tech(...)`.
    #[must_use]
    pub const fn keyword(self) -> &'static str {
        match self {
            Self::ColorCbdt => "color-cbdt",
            Self::ColorColrV0 => "color-colrv0",
            Self::ColorColrV1 => "color-colrv1",
            Self::ColorSbix => "color-sbix",
            Self::ColorSvg => "color-svg",
            Self::FeaturesAat => "features-aat",
            Self::FeaturesGraphite => "features-graphite",
            Self::FeaturesOpenType => "features-opentype",
            Self::Incremental => "incremental",
            Self::Palettes => "palettes",
            Self::Variations => "variations",
        }
    }

    /// A human-readable description of this technology.
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::ColorCbdt => "Color bitmap data tables",
            Self::ColorColrV0 => "Multi-colored glyphs via COLR version 0 table",
            Self::ColorColrV1 => "Multi-colored glyphs via COLR version 1 table",
            Self::ColorSbix => "Standard bitmap graphics tables",
            Self::ColorSvg => "SVG multi-colored tables",
            Self::FeaturesAat => "TrueType morx and kerx tables",
            Self::FeaturesGraphite => {
                "Graphite features, namely Silf, Glat, Gloc, Feat, and Sill tables"
            }
            Self::FeaturesOpenType => "OpenType GSUB and GPOS tables",
            Self::Incremental => "Incremental font loading",
            Self::Palettes => {
                "Font palettes by means of font-palette to select one of many \
                 color palettes in the font"
            }
            Self::Variations => {
                "Font variations in TrueType and OpenType fonts to control the \
                 font axis, weight, glyphs, etc."
            }
        }
    }

    /// Folds this technology into a running content hash.
    pub(crate) const fn hash(self, h: u64) -> u64 {
        topcoat_core::runtime::fnv1a::hash_continue(h, self.keyword().as_bytes())
    }
}

impl std::fmt::Display for FontTech {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.keyword())
    }
}
