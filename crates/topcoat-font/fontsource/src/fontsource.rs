//! Generated metadata for every font family in the vendored Fontsource
//! catalog.
//!
//! Each family is a [`Family`] constant named after its id in
//! `SCREAMING_SNAKE_CASE` (e.g. [`ROBOTO`], [`JETBRAINS_MONO`]). Iterate the
//! whole catalog through [`ALL`], or look a family up by id with [`by_id`].

use crate::Style;

/// Static metadata describing a single font family in the Fontsource catalog.
///
/// One [`Family`] constant is generated for every family in the
/// vendored catalog (see the [`families`](crate::families) module), and
/// the full list is available as [`families::ALL`](crate::families::ALL).
/// The values mirror the fields of the Fontsource [`/v1/fonts`](https://api.fontsource.org/v1/fonts) endpoint.
#[derive(Debug, Clone, Copy)]
pub struct FontsourceFamily {
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
}

impl FontsourceFamily {
    /// Whether the family offers the given weight.
    #[must_use]
    pub fn has_weight(&self, weight: u16) -> bool {
        self.weights.contains(&weight)
    }

    /// Whether the family offers the given style.
    #[must_use]
    pub fn has_style(&self, style: Style) -> bool {
        self.styles.contains(&style)
    }

    /// Whether the family offers the given subset.
    #[must_use]
    pub fn has_subset(&self, subset: Subset) -> bool {
        self.subsets.contains(&subset)
    }

    /// Look up a family by its Fontsource [`id`](Family::id), e.g.
    /// `"roboto"`. Returns `None` if no such family is in the vendored catalog.
    #[must_use]
    pub fn by_id(id: &str) -> Option<&'static FontsourceFamily> {
        ALL.binary_search_by(|f| f.id.cmp(id)).ok().map(|i| ALL[i])
    }
}

include!(concat!(env!("OUT_DIR"), "/fontsource.rs"));
