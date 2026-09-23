use crate::FontStyle;

/// Whether a catalog face is upright or italic.
///
/// Represents the `style` axis in the Fontsource catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Style {
    /// An upright face (`normal`).
    Normal,
    /// An italic face (`italic`).
    Italic,
}

impl Style {
    /// The Fontsource style id, e.g. `"normal"`, as used in CDN file names.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
        }
    }

    /// The corresponding CSS [`FontStyle`].
    #[must_use]
    pub const fn as_font_style(self) -> FontStyle {
        match self {
            Self::Normal => FontStyle::Normal,
            Self::Italic => FontStyle::Italic,
        }
    }
}
