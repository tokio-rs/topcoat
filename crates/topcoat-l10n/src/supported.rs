use crate::Locale;

/// The locales an application serves, and the fallback used when a request
/// matches none of them.
///
/// Create one with [`SupportedLocales::new`] or from an array of locales. The
/// first locale is the fallback unless [`with_fallback`](Self::with_fallback)
/// sets another. Every locale is [canonicalized](Locale::canonicalize), so a
/// supported `iw` is stored as `he`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedLocales {
    locales: Vec<Locale>,
    fallback: Locale,
}

impl SupportedLocales {
    /// Creates the supported locales in order of preference. Earlier locales
    /// win when a request could match several.
    ///
    /// # Panics
    ///
    /// Panics when `locales` is empty.
    #[must_use]
    #[track_caller]
    pub fn new(locales: impl IntoIterator<Item = Locale>) -> Self {
        let mut supported = Vec::new();
        for locale in locales {
            let locale = locale.canonicalize();
            if !supported.contains(&locale) {
                supported.push(locale);
            }
        }
        let Some(fallback) = supported.first().cloned() else {
            panic!("no supported locales: pass at least one to `SupportedLocales::new`");
        };
        Self {
            locales: supported,
            fallback,
        }
    }

    /// Returns these locales with `locale` as the fallback, adding it to the
    /// end of the supported locales if it is missing.
    #[must_use]
    pub fn with_fallback(mut self, locale: Locale) -> Self {
        let locale = locale.canonicalize();
        if !self.locales.contains(&locale) {
            self.locales.push(locale.clone());
        }
        self.fallback = locale;
        self
    }

    /// Returns the supported locales in order of preference. The fallback is
    /// always among them.
    #[must_use]
    pub fn locales(&self) -> &[Locale] {
        &self.locales
    }

    /// Returns the locale used when a request matches none of the supported
    /// locales.
    #[must_use]
    pub const fn fallback(&self) -> &Locale {
        &self.fallback
    }
}

impl<const N: usize> From<[Locale; N]> for SupportedLocales {
    #[track_caller]
    fn from(locales: [Locale; N]) -> Self {
        Self::new(locales)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    #[test]
    fn first_locale_is_the_default_fallback() {
        let supported = SupportedLocales::from([locale!("de"), locale!("en")]);

        assert_eq!(supported.fallback(), &locale!("de"));
        assert_eq!(supported.locales(), [locale!("de"), locale!("en")]);
    }

    #[test]
    fn a_missing_fallback_is_added_last() {
        let supported = SupportedLocales::new([locale!("de")]).with_fallback(locale!("en"));

        assert_eq!(supported.fallback(), &locale!("en"));
        assert_eq!(supported.locales(), [locale!("de"), locale!("en")]);
    }

    #[test]
    fn a_supported_fallback_keeps_its_position() {
        let supported =
            SupportedLocales::new([locale!("de"), locale!("en")]).with_fallback(locale!("en"));

        assert_eq!(supported.fallback(), &locale!("en"));
        assert_eq!(supported.locales(), [locale!("de"), locale!("en")]);
    }

    #[test]
    fn locales_are_canonicalized() {
        let supported =
            SupportedLocales::new([locale!("iw"), locale!("en")]).with_fallback(locale!("en-UK"));

        assert_eq!(supported.fallback(), &locale!("en-GB"));
        assert_eq!(
            supported.locales(),
            [locale!("he"), locale!("en"), locale!("en-GB")]
        );
    }

    #[test]
    fn aliases_of_the_same_locale_are_listed_once() {
        let supported = SupportedLocales::new([locale!("he"), locale!("iw")]);

        assert_eq!(supported.locales(), [locale!("he")]);
    }

    #[test]
    #[should_panic(expected = "no supported locales")]
    fn creating_without_locales_panics() {
        let _ = SupportedLocales::new([]);
    }
}
