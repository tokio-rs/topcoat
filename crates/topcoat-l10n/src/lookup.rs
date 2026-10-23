use std::iter;

use icu_locale::fallback::{LocaleFallbackConfig, LocaleFallbacker, LocaleFallbackerWithConfig};
use icu_locale_core::DataLocale;

use crate::{LanguageIdentifier, Locale};

/// Finds an available locale that can provide translations for `locale`. Returns `None`
/// if there is no match.
///
/// Searches the locale's fallback chain from most specific to least specific, following
/// CLDR rules and excluding the root locale. For example, `de-CH-1996` tries `de-CH`,
/// then `de-1996`, then `de`. Fallback preserves the script, so `zh-TW` can use
/// `zh-Hant` but not `zh`.
///
/// CLDR also defines parent locales. For example, `es-MX` tries `es-419` before `es`,
/// and `pt-AO` tries `pt-PT` before `pt`. Lookup does not choose a sibling or a more
/// specific locale. An available `en-US` therefore cannot serve `en` or `en-GB`.
///
/// Ignores extensions and [canonicalizes](Locale::canonicalize) names before comparing
/// them, so `iw` can use `he`.
///
/// ```
/// use topcoat_l10n::{locale, lookup};
///
/// let available = [locale!("de"), locale!("zh-Hant")];
/// assert_eq!(lookup(&locale!("de-AT"), &available), Some(&locale!("de")));
/// assert_eq!(
///     lookup(&locale!("zh-TW"), &available),
///     Some(&locale!("zh-Hant"))
/// );
/// assert_eq!(lookup(&locale!("fr"), &available), None);
/// ```
#[must_use]
pub fn lookup<'a>(locale: &Locale, available: &'a [Locale]) -> Option<&'a Locale> {
    let normalized: Vec<_> = available
        .iter()
        .map(|locale| normalize_for_lookup(locale.id()))
        .collect();
    lookup_index(locale, &normalized).map(|index| &available[index])
}

/// Returns the index selected by [`lookup`]. Normalize the input locales with
/// [`normalize_for_lookup`] first.
pub(crate) fn lookup_index(locale: &Locale, normalized: &[DataLocale]) -> Option<usize> {
    lookup_indices(locale, normalized).next()
}

/// Lists matching locale indices in fallback order, from most specific to least
/// specific. The first result is the one returned by [`lookup_index`].
pub(crate) fn lookup_indices(
    locale: &Locale,
    normalized: &[DataLocale],
) -> impl Iterator<Item = usize> {
    let id = locale.id().clone().canonicalize();
    let mut chain = fallbacker().fallback_for(DataLocale::from(id.as_icu()));
    iter::from_fn(move || {
        // The chain ends with the root locale, `und`, which serves no locale.
        while !chain.get().language.is_unknown() {
            let index = normalized.iter().position(|step| step == chain.get());
            chain.step();
            if index.is_some() {
                return index;
            }
        }
        None
    })
}

/// Normalizes an identifier for comparisons in [`lookup_index`].
pub(crate) fn normalize_for_lookup(id: &LanguageIdentifier) -> DataLocale {
    // The first step of a chain is the locale with implied subtags removed,
    // such as `zh` for `zh-Hans`, so both sides are compared in that form.
    let id = id.clone().canonicalize();
    fallbacker()
        .fallback_for(DataLocale::from(id.as_icu()))
        .take()
}

fn fallbacker() -> LocaleFallbackerWithConfig<'static> {
    LocaleFallbacker::new().for_config(LocaleFallbackConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    #[test]
    fn prefers_the_locale_itself() {
        assert_eq!(
            lookup(&locale!("de-AT"), &[locale!("de"), locale!("de-AT")]),
            Some(&locale!("de-AT"))
        );
    }

    #[test]
    fn drops_subtags_from_the_most_specific() {
        let available = [locale!("de"), locale!("de-CH")];

        assert_eq!(
            lookup(&locale!("de-CH-1996"), &available),
            Some(&locale!("de-CH"))
        );
        assert_eq!(lookup(&locale!("de-AT"), &available), Some(&locale!("de")));
    }

    #[test]
    fn keeps_the_script() {
        assert_eq!(
            lookup(&locale!("zh-TW"), &[locale!("zh"), locale!("zh-Hant")]),
            Some(&locale!("zh-Hant"))
        );
        assert_eq!(lookup(&locale!("zh-TW"), &[locale!("zh")]), None);
        assert_eq!(lookup(&locale!("sr-Latn"), &[locale!("sr")]), None);
    }

    #[test]
    fn matches_available_locales_with_implied_scripts() {
        assert_eq!(
            lookup(&locale!("zh-CN"), &[locale!("zh-Hans")]),
            Some(&locale!("zh-Hans"))
        );
    }

    #[test]
    fn follows_the_parent_locales_of_cldr() {
        assert_eq!(
            lookup(&locale!("es-MX"), &[locale!("es"), locale!("es-419")]),
            Some(&locale!("es-419"))
        );
        assert_eq!(
            lookup(&locale!("pt-AO"), &[locale!("pt"), locale!("pt-PT")]),
            Some(&locale!("pt-PT"))
        );
    }

    #[test]
    fn does_not_round_to_a_sibling_or_a_more_specific_locale() {
        assert_eq!(lookup(&locale!("en-GB"), &[locale!("en-US")]), None);
        assert_eq!(lookup(&locale!("en"), &[locale!("en-US")]), None);
        assert_eq!(
            lookup(&locale!("en-GB"), &[locale!("en-US"), locale!("en")]),
            Some(&locale!("en"))
        );
    }

    #[test]
    fn ignores_extensions() {
        assert_eq!(
            lookup(&locale!("de-AT-u-ca-buddhist"), &[locale!("de")]),
            Some(&locale!("de"))
        );
    }

    #[test]
    fn compares_canonical_locales() {
        assert_eq!(
            lookup(&locale!("iw"), &[locale!("he")]),
            Some(&locale!("he"))
        );
        assert_eq!(
            lookup(&locale!("he"), &[locale!("iw")]),
            Some(&locale!("iw"))
        );
    }

    #[test]
    fn returns_none_without_a_match() {
        assert_eq!(lookup(&locale!("fr"), &[locale!("de")]), None);
        assert_eq!(lookup(&locale!("de"), &[]), None);
    }
}
