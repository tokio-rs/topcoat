use std::{borrow::Cow, sync::Arc};

use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};
use icu_locale_core::DataLocale;

use crate::{Locale, format_value, lookup_index, lookup_indices, normalize_for_lookup};

/// A collection of Fluent messages translated into several locales.
///
/// [`lookup`](crate::lookup) chooses the locale used to format each message. Missing
/// messages come from that locale's parents. For example, `de-CH` can use messages from
/// `de` and only needs to define its differences.
pub struct Catalog {
    locales: Vec<Locale>,
    normalized: Vec<DataLocale>,
    bundles: Vec<LocaleBundle>,
}

impl Catalog {
    /// Builds a catalog from each locale's Fluent source strings.
    ///
    /// # Panics
    ///
    /// Panics if locale names are equivalent after normalization, such as `zh` and
    /// `zh-Hans`. Also panics on invalid Fluent syntax or duplicate message or term
    /// definitions within a locale.
    #[must_use]
    #[track_caller]
    pub fn new<'a>(locales: impl IntoIterator<Item = (Locale, &'a [&'a str])>) -> Self {
        let mut catalog = Self {
            locales: Vec::new(),
            normalized: Vec::new(),
            bundles: Vec::new(),
        };
        let mut resources = Vec::new();
        for (locale, sources) in locales {
            let locale = locale.canonicalize();
            let normalized = normalize_for_lookup(locale.id());
            if let Some(index) = catalog
                .normalized
                .iter()
                .position(|other| *other == normalized)
            {
                panic!(
                    "the locales `{}` and `{locale}` of a catalog are the same",
                    catalog.locales[index]
                );
            }
            let sources: Vec<_> = sources
                .iter()
                .map(|source| LocaleBundle::parse(&locale, source))
                .collect();
            resources.push(sources);
            catalog.normalized.push(normalized);
            catalog.locales.push(locale);
        }
        catalog.bundles = catalog
            .locales
            .iter()
            .enumerate()
            .map(|(index, locale)| {
                // The first locale that could serve a locale of the catalog is
                // the locale itself, so the rest are its parents.
                let parents = lookup_indices(locale, &catalog.normalized)
                    .skip(1)
                    .map(|parent| &resources[parent][..]);
                LocaleBundle::new(locale, &resources[index], parents)
            })
            .collect();
        catalog
    }

    /// Lists the locales available in this catalog.
    #[must_use]
    pub fn locales(&self) -> &[Locale] {
        &self.locales
    }

    /// Formats a message or attribute using `args`.
    ///
    /// Uses [`lookup`](crate::lookup) to choose a locale. Returns `None` if no locale
    /// matches or the selected locale has no matching message or attribute.
    #[must_use]
    pub fn format(
        &self,
        locale: &Locale,
        id: &str,
        attribute: Option<&str>,
        args: Option<&FluentArgs<'_>>,
    ) -> Option<Cow<'_, str>> {
        let index = lookup_index(locale, &self.normalized)?;
        self.bundles[index].format(id, attribute, args)
    }
}

/// One catalog locale's Fluent bundle.
struct LocaleBundle(FluentBundle<Arc<FluentResource>>);

impl LocaleBundle {
    /// Builds a bundle from the locale's own sources, then fills missing messages from
    /// its parents, starting with the closest parent.
    #[track_caller]
    fn new<'a>(
        locale: &Locale,
        sources: &[Arc<FluentResource>],
        parents: impl IntoIterator<Item = &'a [Arc<FluentResource>]>,
    ) -> Self {
        let id = locale
            .id()
            .to_string()
            .parse()
            .expect("a language identifier is valid in `unic-langid`");
        let mut bundle = FluentBundle::new_concurrent(vec![id]);
        bundle.set_formatter(Some(format_value));
        bundle
            .add_builtins()
            .expect("the builtin functions are added to a new bundle");
        for resource in sources {
            assert!(
                bundle.add_resource(Arc::clone(resource)).is_ok(),
                "a Fluent source of `{locale}` defines a message or term twice"
            );
        }
        for resource in parents.into_iter().flatten() {
            // Fluent keeps the message that was added first and reports the
            // other one as an error, which is how the locale's own messages
            // win over those of its parents.
            let _ = bundle.add_resource(Arc::clone(resource));
        }
        Self(bundle)
    }

    /// Parses Fluent source text for the given locale.
    #[track_caller]
    fn parse(locale: &Locale, source: &str) -> Arc<FluentResource> {
        let resource = FluentResource::try_new(source.to_owned())
            .unwrap_or_else(|_| panic!("a Fluent source of `{locale}` has syntax errors"));
        Arc::new(resource)
    }

    /// Formats a message or attribute, returning `None` if it is unavailable.
    fn format(
        &self,
        id: &str,
        attribute: Option<&str>,
        args: Option<&FluentArgs<'_>>,
    ) -> Option<Cow<'_, str>> {
        let message = self.0.get_message(id)?;
        let pattern = match attribute {
            Some(attribute) => message.get_attribute(attribute)?.value(),
            None => message.value()?,
        };
        // Errors come from arguments that do not fit the message. The text
        // still shows the message, with the faulty parts marked.
        let mut errors = Vec::new();
        Some(self.0.format_pattern(pattern, args, &mut errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale;

    const EN: &str = "\
title = Welcome
items = { $count ->
    [one] One item
   *[other] { $count } items
}
login-input = Log in
    .placeholder = Your name
only-english = Only in English
";

    const DE: &str = "\
title = Willkommen
items = { $count ->
    [one] Ein Artikel
   *[other] { $count } Artikel
}
login-input = Anmelden
";

    fn catalog() -> Catalog {
        Catalog::new([(locale!("en"), &[EN][..]), (locale!("de"), &[DE][..])])
    }

    fn count(count: f64) -> FluentArgs<'static> {
        let mut args = FluentArgs::new();
        args.set("count", count);
        args
    }

    #[test]
    fn formats_in_the_locale_that_serves_the_requested_one() {
        let catalog = catalog();
        assert_eq!(
            catalog
                .format(&locale!("de-AT"), "title", None, None)
                .as_deref(),
            Some("Willkommen")
        );
        assert_eq!(
            catalog
                .format(&locale!("en-US"), "title", None, None)
                .as_deref(),
            Some("Welcome")
        );
    }

    #[test]
    fn returns_none_when_no_locale_serves_the_requested_one() {
        let catalog = catalog();
        assert_eq!(catalog.format(&locale!("ja"), "title", None, None), None);
    }

    #[test]
    fn returns_none_for_a_message_the_locale_lacks() {
        let catalog = catalog();
        assert_eq!(
            catalog.format(&locale!("de"), "only-english", None, None),
            None
        );
        assert_eq!(
            catalog.format(&locale!("de"), "login-input", Some("placeholder"), None),
            None
        );
        assert_eq!(catalog.format(&locale!("de"), "missing", None, None), None);
    }

    #[test]
    fn formats_attributes() {
        let catalog = catalog();
        assert_eq!(
            catalog
                .format(&locale!("en"), "login-input", Some("placeholder"), None)
                .as_deref(),
            Some("Your name")
        );
    }

    #[test]
    fn formats_arguments_for_the_locale() {
        let catalog = catalog();
        let args = count(1234.0);
        let text = catalog
            .format(&locale!("de"), "items", None, Some(&args))
            .expect("German has the message");
        assert!(
            text.contains("1.234") && text.ends_with(" Artikel"),
            "{text}"
        );
        let args = count(1.0);
        assert_eq!(
            catalog
                .format(&locale!("de"), "items", None, Some(&args))
                .as_deref(),
            Some("Ein Artikel")
        );
    }

    #[test]
    fn combines_the_sources_of_a_locale() {
        let catalog = Catalog::new([(locale!("en"), &["a = A\n", "b = B\n"][..])]);
        assert_eq!(
            catalog.format(&locale!("en"), "a", None, None).as_deref(),
            Some("A")
        );
        assert_eq!(
            catalog.format(&locale!("en"), "b", None, None).as_deref(),
            Some("B")
        );
    }

    #[test]
    fn a_regional_locale_takes_the_messages_it_lacks_from_its_parent() {
        let catalog = Catalog::new([
            (locale!("de"), &[DE][..]),
            (locale!("de-CH"), &["title = Gruezi\n"][..]),
        ]);
        assert_eq!(
            catalog
                .format(&locale!("de-CH"), "title", None, None)
                .as_deref(),
            Some("Gruezi")
        );
        assert_eq!(
            catalog
                .format(&locale!("de-CH"), "login-input", None, None)
                .as_deref(),
            Some("Anmelden")
        );
        // The parent keeps its own messages.
        assert_eq!(
            catalog
                .format(&locale!("de"), "title", None, None)
                .as_deref(),
            Some("Willkommen")
        );
    }

    #[test]
    fn a_regional_locale_does_not_take_messages_from_a_sibling() {
        let catalog = Catalog::new([
            (locale!("en"), &["color = Color\n"][..]),
            (locale!("en-GB"), &["color = Colour\n"][..]),
            (locale!("en-US"), &["title = Welcome\n"][..]),
        ]);
        assert_eq!(
            catalog
                .format(&locale!("en-US"), "color", None, None)
                .as_deref(),
            Some("Color")
        );
    }

    #[test]
    fn the_closest_parent_wins() {
        // CLDR makes `en-001` the parent of `en-AU`, and `en` its grandparent.
        let catalog = Catalog::new([
            (locale!("en"), &["color = Color\n"][..]),
            (locale!("en-001"), &["color = Colour\n"][..]),
            (locale!("en-AU"), &["title = Welcome\n"][..]),
        ]);
        assert_eq!(
            catalog
                .format(&locale!("en-AU"), "color", None, None)
                .as_deref(),
            Some("Colour")
        );
    }

    #[test]
    #[should_panic(expected = "twice")]
    fn panics_when_a_locale_defines_a_message_twice() {
        let _ = Catalog::new([(locale!("en"), &["a = A\n", "a = B\n"][..])]);
    }

    #[test]
    #[should_panic(expected = "are the same")]
    fn panics_when_two_locales_are_the_same() {
        let _ = Catalog::new([(locale!("zh"), &[][..]), (locale!("zh-Hans"), &[][..])]);
    }
}
