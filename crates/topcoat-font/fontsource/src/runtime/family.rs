//! Generated metadata for every font family in the vendored Fontsource
//! catalog.
//!
//! Each family is a [`Family`] constant named after its id in
//! `SCREAMING_SNAKE_CASE` (e.g. [`ROBOTO`], [`JETBRAINS_MONO`]). Iterate the
//! whole catalog through [`ALL`], or look a family up by id with
//! [`Family::by_id`].

use topcoat_font::runtime::{UnicodeRange, UnicodeRanges};

use crate::runtime::{Style, Subset};

/// Static metadata describing a single font family in the Fontsource catalog.
///
/// One [`Family`] constant is generated for every family in the vendored
/// catalog, and the full list is available as [`ALL`]. The values mirror the
/// fields of the Fontsource [`/v1/fonts`](https://api.fontsource.org/v1/fonts)
/// endpoint.
#[derive(Debug, Clone, Copy)]
pub struct Family {
    /// Fontsource id, used to build CDN URLs (e.g. `"roboto"`).
    pub id: &'static str,
    /// Human-readable family name (e.g. `"Roboto"`).
    pub name: &'static str,
    /// Character subsets the family ships, such as
    /// [`Subset::Latin`] or [`Subset::Cyrillic`].
    pub subsets: &'static [Subset],
    /// Numeric weights available, from `100` to `900`.
    pub weights: &'static [u16],
    /// Styles available ([`Normal`](Style::Normal) and/or
    /// [`Italic`](Style::Italic)).
    pub styles: &'static [Style],
    /// The subset served when none is requested.
    pub default_subset: Subset,
    /// Whether the family ships as a variable font.
    pub variable: bool,
    /// Typographic category, such as `"sans-serif"` or `"monospace"`.
    pub category: &'static str,
    /// SPDX license identifier (e.g. `"OFL-1.1"`).
    pub license: &'static str,
    /// Upstream source the family is mirrored from (e.g. `"google"`).
    pub provider: &'static str,
    /// The `unicode-range` each named subset covers, paired with its [`Subset`].
    ///
    /// Numbered CJK subset blocks are omitted, so this can be shorter than
    /// [`subsets`](Self::subsets); look a subset up with
    /// [`unicode_range`](Self::unicode_range).
    pub unicode_ranges: &'static [(Subset, UnicodeRanges)],
}

impl Family {
    /// Whether the family offers the given weight.
    #[must_use]
    pub const fn has_weight(&self, weight: u16) -> bool {
        let mut i = 0;
        while i < self.weights.len() {
            if self.weights[i] == weight {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Whether the family offers the given style.
    #[must_use]
    pub const fn has_style(&self, style: Style) -> bool {
        let mut i = 0;
        while i < self.styles.len() {
            if self.styles[i] as u16 == style as u16 {
                return true;
            }
            i += 1;
        }
        false
    }

    /// Whether the family offers the given subset.
    #[must_use]
    pub const fn has_subset(&self, subset: Subset) -> bool {
        let mut i = 0;
        while i < self.subsets.len() {
            if self.subsets[i] as u16 == subset as u16 {
                return true;
            }
            i += 1;
        }
        false
    }

    /// The `unicode-range` this family ships for `subset`, if known.
    ///
    /// Returns `None` for subsets without vendored ranges — notably the
    /// numbered CJK blocks — in which case a face for that subset is emitted
    /// without a `unicode-range` descriptor.
    #[must_use]
    pub const fn unicode_range(&self, subset: Subset) -> Option<UnicodeRanges> {
        let mut i = 0;
        while i < self.unicode_ranges.len() {
            if self.unicode_ranges[i].0 as u16 == subset as u16 {
                return Some(self.unicode_ranges[i].1);
            }
            i += 1;
        }
        None
    }

    /// Look up a family by its Fontsource [`id`](Family::id), e.g. `"roboto"`.
    ///
    /// Returns `None` if no such family is in the vendored catalog.
    #[must_use]
    pub fn by_id(id: &str) -> Option<&'static Family> {
        ALL.binary_search_by(|f| f.id.cmp(id)).ok().map(|i| ALL[i])
    }
}

include!(concat!(env!("OUT_DIR"), "/families.rs"));
