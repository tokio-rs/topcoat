use std::fmt::{self, Debug, Display};

/// The extensions of a [`Locale`](crate::Locale), such as the Unicode
/// extension `u-ca-buddhist` that selects the Buddhist calendar.
///
/// Extensions refine how a locale formats and processes text without changing
/// its language. They are written after the language identifier, each
/// introduced by a single-letter subtag: `u` for Unicode extensions, `t` for
/// transformed content, and `x` for private use.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Extensions(icu_locale_core::extensions::Extensions);

impl Extensions {
    /// Creates an empty set of extensions.
    #[must_use]
    pub const fn new() -> Self {
        Self(icu_locale_core::extensions::Extensions::new())
    }

    /// Returns whether there are no extensions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Borrows the `icu_locale_core` extensions.
    #[must_use]
    pub const fn as_icu(&self) -> &icu_locale_core::extensions::Extensions {
        &self.0
    }

    /// Converts to the `icu_locale_core` extensions.
    #[must_use]
    pub fn into_icu(self) -> icu_locale_core::extensions::Extensions {
        self.0
    }

    /// Converts from the `icu_locale_core` extensions.
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
