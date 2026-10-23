use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use crate::LocaleParseError;

/// A language variant such as `posix` or `1996`.
///
/// A variant contains five to eight ASCII letters or digits, or four such characters
/// starting with a digit. Use [`variant!`](crate::variant) for fixed text or
/// [`str::parse`] for runtime input. Parsing converts letters to lowercase.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant(icu_locale_core::subtags::Variant);

impl Variant {
    /// Parses a variant and converts letters to lowercase.
    ///
    /// # Errors
    ///
    /// Returns an error unless the input contains five to eight ASCII letters or
    /// digits, or four such characters starting with a digit.
    pub const fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        match icu_locale_core::subtags::Variant::try_from_str(text) {
            Ok(variant) => Ok(Self(variant)),
            Err(_) => Err(LocaleParseError::InvalidVariant),
        }
    }

    /// Returns the normalized variant text.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Returns the variant as an `icu_locale_core` subtag.
    #[must_use]
    pub const fn to_icu(self) -> icu_locale_core::subtags::Variant {
        self.0
    }

    /// Wraps a variant subtag from `icu_locale_core`.
    #[must_use]
    pub const fn from_icu(variant: icu_locale_core::subtags::Variant) -> Self {
        Self(variant)
    }
}

impl AsRef<str> for Variant {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Debug for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Variant").field(&self.as_str()).finish()
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Variant {
    type Err = LocaleParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(text)
    }
}

/// Checks a fixed variant at compile time and creates a [`Variant`].
///
/// Invalid variants cause a compile error. Works in `const` and `static` items.
#[macro_export]
macro_rules! variant {
    ($text:literal) => {
        const {
            match $crate::Variant::try_from_str($text) {
                Ok(variant) => variant,
                Err(_) => panic!(concat!("invalid variant subtag: ", $text)),
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalizes_to_lower_case() {
        assert_eq!("POSIX".parse::<Variant>().unwrap(), variant!("posix"));
        assert_eq!("1996".parse::<Variant>().unwrap().to_string(), "1996");
    }

    #[test]
    fn rejects_malformed_text() {
        assert_eq!(
            "abcd".parse::<Variant>(),
            Err(LocaleParseError::InvalidVariant)
        );
        assert!("valencian".parse::<Variant>().is_err());
    }
}
