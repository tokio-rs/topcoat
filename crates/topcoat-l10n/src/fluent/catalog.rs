use std::borrow::Cow;

use fluent_bundle::{FluentArgs, FluentResource, concurrent::FluentBundle};

use crate::{Locale, SupportedLocales, format_value};

/// The Fluent messages of an application or one of its parts, translated into
/// several locales.
///
/// Each locale has its own messages. A locale does not need to translate every
/// message: a message it lacks is taken from the fallback locale.
pub struct Catalog {
    supported: SupportedLocales,
    bundles: Vec<(Locale, LocaleBundle)>,
}

impl Catalog {
    /// Creates a catalog from the Fluent sources of each locale.
    ///
    /// The locales become the catalog's
    /// [supported locales](Self::supported_locales), with `fallback` as their
    /// fallback.
    ///
    /// # Panics
    ///
    /// Panics when `fallback` has no sources, or when a source has syntax
    /// errors or defines a message or term that the locale already defines.
    #[must_use]
    #[track_caller]
    pub fn new<'a>(
        fallback: Locale,
        locales: impl IntoIterator<Item = (Locale, &'a [&'a str])>,
    ) -> Self {
        let bundles: Vec<_> = locales
            .into_iter()
            .map(|(locale, sources)| {
                let locale = locale.canonicalize();
                let bundle = LocaleBundle::new(&locale, sources);
                (locale, bundle)
            })
            .collect();
        let fallback = fallback.canonicalize();
        assert!(
            bundles.iter().any(|(locale, _)| *locale == fallback),
            "the fallback locale `{fallback}` has no Fluent sources"
        );
        let supported = SupportedLocales::new(bundles.iter().map(|(locale, _)| locale.clone()))
            .with_fallback(fallback);
        Self { supported, bundles }
    }

    /// Returns the locales the catalog translates into, and its fallback.
    #[must_use]
    pub const fn supported_locales(&self) -> &SupportedLocales {
        &self.supported
    }

    /// Formats a message, or one of its attributes, with `args`.
    ///
    /// The message is taken from the supported locale that best matches
    /// `requested`, as chosen by [`SupportedLocales::negotiate`]. If that
    /// locale lacks the message, it is taken from the fallback locale. If the
    /// fallback lacks it too, the result is `id`.
    pub fn format<'a, 'r>(
        &'a self,
        requested: impl IntoIterator<Item = &'r Locale>,
        id: &'a str,
        attribute: Option<&str>,
        args: Option<&FluentArgs<'_>>,
    ) -> Cow<'a, str> {
        let locale = self.supported.negotiate(requested);
        for locale in [locale, self.supported.fallback()] {
            if let Some(bundle) = self.bundle(locale)
                && let Some(text) = bundle.format(id, attribute, args)
            {
                return text;
            }
        }
        Cow::Borrowed(id)
    }

    /// Returns the bundle of a supported locale.
    fn bundle(&self, locale: &Locale) -> Option<&LocaleBundle> {
        self.bundles
            .iter()
            .find(|(candidate, _)| candidate == locale)
            .map(|(_, bundle)| bundle)
    }
}

/// The messages of one locale of a [`Catalog`].
struct LocaleBundle(FluentBundle<FluentResource>);

impl LocaleBundle {
    /// Creates the messages of `locale` from its Fluent sources.
    #[track_caller]
    fn new(locale: &Locale, sources: &[&str]) -> Self {
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
        for source in sources {
            let resource = FluentResource::try_new((*source).to_owned())
                .unwrap_or_else(|_| panic!("a Fluent source of `{locale}` has syntax errors"));
            if bundle.add_resource(resource).is_err() {
                panic!("a Fluent source of `{locale}` defines a message or term twice");
            }
        }
        Self(bundle)
    }

    /// Formats a message, or one of its attributes, or returns `None` when
    /// this locale lacks it.
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
        Catalog::new(
            locale!("en"),
            [(locale!("en"), &[EN][..]), (locale!("de"), &[DE][..])],
        )
    }

    fn count(count: f64) -> FluentArgs<'static> {
        let mut args = FluentArgs::new();
        args.set("count", count);
        args
    }

    #[test]
    fn formats_in_the_best_matching_locale() {
        let catalog = catalog();
        assert_eq!(catalog.format(&[locale!("de-AT")], "title", None, None), "Willkommen");
        assert_eq!(catalog.format(&[locale!("en-US")], "title", None, None), "Welcome");
        assert_eq!(
            catalog.format(&[locale!("fr"), locale!("de")], "title", None, None),
            "Willkommen"
        );
    }

    #[test]
    fn uses_the_fallback_when_no_locale_matches() {
        let catalog = catalog();
        assert_eq!(catalog.format(&[locale!("ja")], "title", None, None), "Welcome");
        assert_eq!(catalog.format(&[], "title", None, None), "Welcome");
    }

    #[test]
    fn takes_missing_messages_from_the_fallback() {
        let catalog = catalog();
        assert_eq!(
            catalog.format(&[locale!("de")], "only-english", None, None),
            "Only in English"
        );
        assert_eq!(
            catalog.format(&[locale!("de")], "login-input", Some("placeholder"), None),
            "Your name"
        );
    }

    #[test]
    fn formats_attributes() {
        let catalog = catalog();
        assert_eq!(
            catalog.format(&[locale!("en")], "login-input", Some("placeholder"), None),
            "Your name"
        );
    }

    #[test]
    fn formats_arguments_for_the_locale() {
        let catalog = catalog();
        let args = count(1234.0);
        let text = catalog.format(&[locale!("de")], "items", None, Some(&args));
        assert!(text.contains("1.234") && text.ends_with(" Artikel"), "{text}");
        let args = count(1.0);
        assert_eq!(catalog.format(&[locale!("de")], "items", None, Some(&args)), "Ein Artikel");
    }

    #[test]
    fn returns_the_id_of_a_message_no_locale_has() {
        let catalog = catalog();
        assert_eq!(catalog.format(&[locale!("de")], "missing", None, None), "missing");
    }

    #[test]
    fn combines_the_sources_of_a_locale() {
        let catalog = Catalog::new(
            locale!("en"),
            [(locale!("en"), &["a = A\n", "b = B\n"][..])],
        );
        assert_eq!(catalog.format(&[], "a", None, None), "A");
        assert_eq!(catalog.format(&[], "b", None, None), "B");
    }

    #[test]
    #[should_panic(expected = "has no Fluent sources")]
    fn panics_when_the_fallback_has_no_sources() {
        let _ = Catalog::new(locale!("fr"), [(locale!("en"), &[EN][..])]);
    }
}
