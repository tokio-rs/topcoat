use crate::{LanguageIdentifier, Locale};

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

    /// Returns the supported locale that best matches `requested`, or the
    /// fallback when none matches.
    ///
    /// Requested locales are tried most preferred first. Each one matches a
    /// supported locale with the same language identifier, then one with the
    /// same language, script, and region after filling in
    /// [likely subtags](crate::LanguageIdentifier::maximize), then one with the
    /// same language and script. So `en-GB` matches a supported `en`, and
    /// `zh-TW` matches `zh-Hant` but not `zh-Hans`.
    #[must_use]
    pub fn negotiate<'a>(&self, requested: impl IntoIterator<Item = &'a Locale>) -> &Locale {
        let maximized: Vec<_> = self
            .locales
            .iter()
            .map(|locale| locale.id().clone().maximize())
            .collect();

        for requested in requested {
            let id = requested.id().clone().canonicalize();
            if let Some(locale) = self.locales.iter().find(|locale| locale.id() == &id) {
                return locale;
            }

            let id = id.maximize();
            let tiers: [fn(&LanguageIdentifier, &LanguageIdentifier) -> bool; 2] = [
                |a, b| {
                    a.language() == b.language()
                        && a.script() == b.script()
                        && a.region() == b.region()
                },
                |a, b| a.language() == b.language() && a.script() == b.script(),
            ];
            for matches in tiers {
                if let Some(index) = maximized
                    .iter()
                    .position(|supported| matches(supported, &id))
                {
                    return &self.locales[index];
                }
            }
        }

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
    fn negotiate_prefers_earlier_requested_locales() {
        let supported = SupportedLocales::from([locale!("en"), locale!("de")]);

        assert_eq!(
            supported.negotiate(&[locale!("fr"), locale!("de"), locale!("en")]),
            &locale!("de")
        );
    }

    #[test]
    fn negotiate_matches_regional_and_script_variants() {
        let supported = SupportedLocales::from([
            locale!("en-US"),
            locale!("en-GB"),
            locale!("zh-Hans"),
            locale!("zh-Hant"),
        ]);

        assert_eq!(supported.negotiate(&[locale!("en-GB")]), &locale!("en-GB"));
        assert_eq!(supported.negotiate(&[locale!("en")]), &locale!("en-US"));
        assert_eq!(supported.negotiate(&[locale!("en-AU")]), &locale!("en-US"));
        assert_eq!(
            supported.negotiate(&[locale!("zh-TW")]),
            &locale!("zh-Hant")
        );
        assert_eq!(
            supported.negotiate(&[locale!("zh-CN")]),
            &locale!("zh-Hans")
        );
    }

    #[test]
    fn negotiate_does_not_match_across_scripts() {
        let supported = SupportedLocales::from([locale!("en"), locale!("sr-Cyrl")]);

        assert_eq!(supported.negotiate(&[locale!("sr-Latn")]), &locale!("en"));
    }

    #[test]
    fn negotiate_matches_deprecated_requested_subtags() {
        let supported = SupportedLocales::from([locale!("en"), locale!("he")]);

        assert_eq!(supported.negotiate(&[locale!("iw")]), &locale!("he"));
    }

    #[test]
    fn negotiate_uses_the_fallback_when_nothing_matches() {
        let supported =
            SupportedLocales::new([locale!("de"), locale!("fr")]).with_fallback(locale!("en"));

        assert_eq!(supported.negotiate(&[locale!("ja")]), &locale!("en"));
        assert_eq!(supported.negotiate(&[]), &locale!("en"));
    }

    #[test]
    #[should_panic(expected = "no supported locales")]
    fn creating_without_locales_panics() {
        let _ = SupportedLocales::new([]);
    }
}
