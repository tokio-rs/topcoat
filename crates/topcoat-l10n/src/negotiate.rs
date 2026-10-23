use crate::{LanguageIdentifier, Locale};

/// Returns the locale of `available` that best matches `requested`, or `None`
/// when none matches.
///
/// Requested locales are tried most preferred first. Each one matches an
/// available locale with the same language identifier, then one with the same
/// language, script, and region after filling in
/// [likely subtags](crate::LanguageIdentifier::maximize), then one with the
/// same language and script. So `en-GB` matches an available `en`, and `zh-TW`
/// matches `zh-Hant` but not `zh-Hans`. Earlier available locales win when a
/// requested locale matches several of them equally well.
///
/// Locales are compared after [canonicalization](Locale::canonicalize), so a
/// requested `iw` matches an available `he`.
///
/// ```
/// use topcoat_l10n::{locale, negotiate};
///
/// let available = [locale!("en"), locale!("de")];
/// let requested = [locale!("fr"), locale!("de-AT")];
/// assert_eq!(negotiate(&requested, &available), Some(&locale!("de")));
/// ```
#[must_use]
pub fn negotiate<'a, 'r>(
    requested: impl IntoIterator<Item = &'r Locale>,
    available: &'a [Locale],
) -> Option<&'a Locale> {
    let canonical: Vec<_> = available
        .iter()
        .map(|locale| locale.id().clone().canonicalize())
        .collect();
    let maximized: Vec<_> = canonical.iter().map(|id| id.clone().maximize()).collect();

    for requested in requested {
        let id = requested.id().clone().canonicalize();
        if let Some(index) = canonical.iter().position(|candidate| *candidate == id) {
            return Some(&available[index]);
        }

        let id = id.maximize();
        let tiers: [fn(&LanguageIdentifier, &LanguageIdentifier) -> bool; 2] = [
            |a, b| {
                a.language() == b.language() && a.script() == b.script() && a.region() == b.region()
            },
            |a, b| a.language() == b.language() && a.script() == b.script(),
        ];
        for matches in tiers {
            if let Some(index) = maximized
                .iter()
                .position(|candidate| matches(candidate, &id))
            {
                return Some(&available[index]);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    #[test]
    fn prefers_earlier_requested_locales() {
        let available = [locale!("en"), locale!("de")];

        assert_eq!(
            negotiate(&[locale!("fr"), locale!("de"), locale!("en")], &available),
            Some(&locale!("de"))
        );
    }

    #[test]
    fn matches_regional_and_script_variants() {
        let available = [
            locale!("en-US"),
            locale!("en-GB"),
            locale!("zh-Hans"),
            locale!("zh-Hant"),
        ];
        let best = |requested| negotiate(&[requested], &available);

        assert_eq!(best(locale!("en-GB")), Some(&locale!("en-GB")));
        assert_eq!(best(locale!("en")), Some(&locale!("en-US")));
        assert_eq!(best(locale!("en-AU")), Some(&locale!("en-US")));
        assert_eq!(best(locale!("zh-TW")), Some(&locale!("zh-Hant")));
        assert_eq!(best(locale!("zh-CN")), Some(&locale!("zh-Hans")));
    }

    #[test]
    fn does_not_match_across_scripts() {
        assert_eq!(
            negotiate(&[locale!("sr-Latn")], &[locale!("en"), locale!("sr-Cyrl")]),
            None
        );
    }

    #[test]
    fn matches_deprecated_subtags_on_either_side() {
        assert_eq!(
            negotiate(&[locale!("iw")], &[locale!("en"), locale!("he")]),
            Some(&locale!("he"))
        );
        assert_eq!(
            negotiate(&[locale!("he")], &[locale!("en"), locale!("iw")]),
            Some(&locale!("iw"))
        );
    }

    #[test]
    fn returns_none_when_nothing_matches() {
        let available = [locale!("de"), locale!("fr")];

        assert_eq!(negotiate(&[locale!("ja")], &available), None);
        assert_eq!(negotiate(&[], &available), None);
        assert_eq!(negotiate(&[locale!("de")], &[]), None);
    }
}
