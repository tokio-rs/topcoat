use std::iter;

use icu_locale::fallback::{LocaleFallbackConfig, LocaleFallbacker, LocaleFallbackerWithConfig};
use icu_locale_core::DataLocale;

use crate::{LanguageIdentifier, Locale};

/// Returns the locale of `available` that serves `locale`, or `None` when none
/// does.
///
/// The locale is looked up along its fallback chain as defined by CLDR, from
/// the most specific locale to the least, without the root locale. Each step
/// drops a subtag without changing the script: `de-CH-1996` is served by
/// `de-CH`, then by `de-1996`, then by `de`, and `zh-TW` is served by
/// `zh-Hant`, but never by `zh`, which is written in another script. The chain
/// follows the parent locales of CLDR, so `es-MX` is served by `es-419` before
/// `es`, and `pt-AO` by `pt-PT` before `pt`. A less specific locale never
/// serves a more specific one, so an available `en-US` does not serve `en` or
/// `en-GB`.
///
/// Extensions are ignored, and locales are compared after
/// [canonicalization](Locale::canonicalize), so `iw` is served by `he`.
///
/// ```
/// use topcoat_l10n::{locale, lookup};
///
/// let available = [locale!("de"), locale!("zh-Hant")];
/// assert_eq!(lookup(&locale!("de-AT"), &available), Some(&locale!("de")));
/// assert_eq!(lookup(&locale!("zh-TW"), &available), Some(&locale!("zh-Hant")));
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

/// Returns the position in `normalized` of the locale that serves `locale`, as
/// chosen by [`lookup`]. Each locale of `normalized` comes from
/// [`normalize_for_lookup`].
pub(crate) fn lookup_index(locale: &Locale, normalized: &[DataLocale]) -> Option<usize> {
    lookup_indices(locale, normalized).next()
}

/// Returns the positions in `normalized` of every locale that could serve
/// `locale`, from the most specific to the least. The first one is the locale
/// chosen by [`lookup_index`].
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

/// Returns `id` in the form that [`lookup_index`] compares available locales
/// in.
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
