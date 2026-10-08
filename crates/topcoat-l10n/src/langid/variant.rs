use std::{
    fmt::{self, Debug, Display},
    str::FromStr,
};

use crate::LocaleParseError;

/// A variant subtag, such as `posix` or `1996`: five to eight ASCII
/// alphanumerics, or a digit followed by three alphanumerics.
///
/// Create one with the [`variant!`](crate::variant) macro for text known at
/// compile time, or parse one with [`str::parse`]. Parsing canonicalizes the
/// text to lower case.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Variant(icu_locale_core::subtags::Variant);

impl Variant {
    /// Parses a variant subtag, canonicalizing its case.
    ///
    /// # Errors
    ///
    /// Fails when the text is neither five to eight ASCII alphanumerics nor
    /// a digit followed by three alphanumerics.
    pub const fn try_from_str(text: &str) -> Result<Self, LocaleParseError> {
        match icu_locale_core::subtags::Variant::try_from_str(text) {
            Ok(variant) => Ok(Self(variant)),
            Err(_) => Err(LocaleParseError::InvalidVariant),
        }
    }

    /// Returns the canonical text of the subtag.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Converts to the `icu_locale_core` subtag.
    #[must_use]
    pub const fn to_icu(self) -> icu_locale_core::subtags::Variant {
        self.0
    }

    /// Converts from the `icu_locale_core` subtag.
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

/// Creates a [`Variant`] from text known at compile time.
///
/// Malformed text is a compile error. The macro is usable in `const` and
/// `static` items.
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
