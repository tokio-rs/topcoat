/// The style axis of a font face: upright or slanted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style {
    /// Upright (`normal`) face.
    Normal,
    /// Slanted (`italic`) face.
    Italic,
}

impl Style {
    /// The CSS `font-style` keyword for this style.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Italic => "italic",
        }
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
