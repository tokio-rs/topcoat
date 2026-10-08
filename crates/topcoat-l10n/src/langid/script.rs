use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use crate::LocaleParseError;

/// A script subtag, such as `Latn` or `Hant`: four ASCII letters.
///
/// Create one with the [`script!`](crate::script) macro for text known at
/// compile time, or parse one with [`str::parse`]. Parsing canonicalizes the
/// text to title case.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Script(icu_locale_core::subtags::Script);

impl Script {
    /// Parses a script subtag, canonicalizing its case.
    ///
    /// # Errors
    ///
    /// Fails when the text is not four ASCII letters.
    pub const fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        match icu_locale_core::subtags::Script::try_from_str(text) {
            Ok(script) => Ok(Self(script)),
            Err(_) => Err(LocaleParseError::InvalidScript),
        }
    }

    /// Returns the canonical text of the subtag.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts to the `icu_locale_core` subtag.
    #[must_use]
    pub const fn to_icu(self) -> icu_locale_core::subtags::Script {
        self.0
    }

    /// Converts from the `icu_locale_core` subtag.
    #[must_use]
    pub const fn from_icu(script: icu_locale_core::subtags::Script) -> Self {
        Self(script)
    }
}

impl AsRef<str> for Script {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Script").field(&self.as_str()).finish()
    }
}

impl Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Script {
    type Err = LocaleParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(text)
    }
}

/// Creates a [`Script`] from text known at compile time.
///
/// Malformed text is a compile error. The macro is usable in `const` and
/// `static` items.
#[macro_export]
macro_rules! script {
    ($text:literal) => {
        const {
            match $crate::Script::try_from_str($text) {
                Ok(script) => script,
                Err(_) => panic!(concat!("invalid script subtag: ", $text)),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_to_title_case() {
        assert_eq!("latn".parse::<Script>().unwrap(), script!("Latn"));
        assert_eq!("HANT".parse::<Script>().unwrap().to_string(), "Hant");
    }

    #[test]
    fn rejects_malformed_text() {
        assert_eq!(
            "Lat".parse::<Script>(),
            Err(LocaleParseError::InvalidScript)
        );
        assert!("La1n".parse::<Script>().is_err());
    }
}
