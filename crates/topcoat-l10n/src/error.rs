use std::fmt::{self, Display};

/// The reason a locale, language identifier, or subtag failed to parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LocaleParseError {
    /// The language subtag is missing or malformed.
    InvalidLanguage,
    /// A script subtag is malformed.
    InvalidScript,
    /// A region subtag is malformed.
    InvalidRegion,
    /// A variant subtag is malformed.
    InvalidVariant,
    /// A subtag fits none of the positions in a language identifier.
    InvalidSubtag,
    /// An extension is malformed.
    InvalidExtension,
    /// The same extension appears twice.
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
