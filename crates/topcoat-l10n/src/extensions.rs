use std::fmt::{self, Debug, Display};

/// Extra settings on a [`Locale`](crate::Locale), such as `u-ca-buddhist` for the
/// Buddhist calendar.
///
/// Extensions describe how to format or process text without changing the language.
/// They follow the language identifier and start with a single-letter subtag. For
/// example, `u` introduces Unicode settings, `t` describes transformed content and `x`
/// marks private-use data.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Extensions(icu_locale_core::extensions::Extensions);

impl Extensions {
    /// Creates extensions with no settings.
    #[must_use]
    pub const fn new() -> Self {
        Self(icu_locale_core::extensions::Extensions::new())
    }

    /// Checks whether the extension set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a reference to the underlying `icu_locale_core` extensions.
    #[must_use]
    pub const fn as_icu(&self) -> &icu_locale_core::extensions::Extensions {
        &self.0
    }

    /// Returns the extensions as an `icu_locale_core` value.
    #[must_use]
    pub fn into_icu(self) -> icu_locale_core::extensions::Extensions {
        self.0
    }

    /// Wraps extensions from `icu_locale_core`.
    #[must_use]
    pub const fn from_icu(extensions: icu_locale_core::extensions::Extensions) -> Self {
        Self(extensions)
    }
}

impl Debug for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Extensions")
            .field(&self.to_string())
            .finish()
    }
}

impl Display for Extensions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}
