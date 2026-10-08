use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use crate::LocaleParseError;

/// A language subtag, such as `en` or `zh`: two or three ASCII letters.
///
/// Create one with the [`language!`](crate::language) macro for text known
/// at compile time, or parse one with [`str::parse`]. Parsing canonicalizes
/// the text to lower case.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Language(icu_locale_core::subtags::Language);

impl Language {
    /// The unknown language, written `und`.
    pub const UNKNOWN: Self = Self(icu_locale_core::subtags::Language::UNKNOWN);

    /// Parses a language subtag, canonicalizing its case.
    ///
    /// # Errors
    ///
    /// Fails when the text is not two or three ASCII letters.
    pub const fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        match icu_locale_core::subtags::Language::try_from_str(text) {
            Ok(language) => Ok(Self(language)),
            Err(_) => Err(LocaleParseError::InvalidLanguage),
        }
    }

    /// Returns the canonical text of the subtag.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns whether this is the unknown language.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        self.0.is_unknown()
    }

    /// Converts to the `icu_locale_core` subtag.
    #[must_use]
    pub const fn to_icu(self) -> icu_locale_core::subtags::Language {
        self.0
    }

    /// Converts from the `icu_locale_core` subtag.
    #[must_use]
    pub const fn from_icu(language: icu_locale_core::subtags::Language) -> Self {
        Self(language)
    }
}

impl AsRef<str> for Language {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Debug for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Language").field(&self.as_str()).finish()
    }
}

impl Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Language {
    type Err = LocaleParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(text)
    }
}

/// Creates a [`Language`] from text known at compile time.
///
/// Malformed text is a compile error. The macro is usable in `const` and
/// `static` items.
#[macro_export]
macro_rules! language {
    ($text:literal) => {
        const {
            match $crate::Language::try_from_str($text) {
                Ok(language) => language,
                Err(_) => panic!(concat!("invalid language subtag: ", $text)),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_to_lower_case() {
        assert_eq!("EN".parse::<Language>().unwrap(), language!("en"));
        assert_eq!("De".parse::<Language>().unwrap().to_string(), "de");
    }

    #[test]
    fn rejects_malformed_text() {
        assert_eq!(
            "e1".parse::<Language>(),
            Err(LocaleParseError::InvalidLanguage)
        );
        assert!("".parse::<Language>().is_err());
    }

    #[test]
    fn recognizes_the_unknown_language() {
        assert!(Language::UNKNOWN.is_unknown());
        assert!("UND".parse::<Language>().unwrap().is_unknown());
        assert!(!language!("en").is_unknown());
    }
}
