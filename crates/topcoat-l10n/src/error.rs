use std::fmt::{self, Display};

/// An error from parsing a locale, language identifier or subtag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocaleParseError {
    /// The language code is absent or invalid.
    InvalidLanguage,
    /// The script code is invalid.
    InvalidScript,
    /// The region code is invalid.
    InvalidRegion,
    /// The variant is invalid.
    InvalidVariant,
    /// A subtag cannot be used at any position in the identifier.
    InvalidSubtag,
    /// An extension has invalid syntax.
    InvalidExtension,
    /// An extension was supplied more than once.
    DuplicateExtension,
}

impl LocaleParseError {
    pub(crate) const fn from_icu(error: icu_locale_core::ParseError) -> Self {
        use icu_locale_core::ParseError;

        match error {
            ParseError::InvalidLanguage => Self::InvalidLanguage,
            ParseError::InvalidExtension => Self::InvalidExtension,
            ParseError::DuplicatedExtension => Self::DuplicateExtension,
            _ => Self::InvalidSubtag,
        }
    }
}

impl Display for LocaleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLanguage => "invalid language subtag",
            Self::InvalidScript => "invalid script subtag",
            Self::InvalidRegion => "invalid region subtag",
            Self::InvalidVariant => "invalid variant subtag",
            Self::InvalidSubtag => "subtag fits no position in the language identifier",
            Self::InvalidExtension => "invalid locale extension",
            Self::DuplicateExtension => "duplicate locale extension",
        })
    }
}

impl std::error::Error for LocaleParseError {}
