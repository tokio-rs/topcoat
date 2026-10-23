use std::{borrow::Cow, fmt::Write, hint};

use fluent_bundle::{FluentArgs, FluentValue};
use topcoat_core::{context::Cx, cursor::ConstReader};
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Locale, Namespace, RawTranslation, try_locale};

/// A message of a `messages!` namespace, or one of its attributes, with its
/// arguments.
///
/// A translation is not formatted until it is [resolved](Self::resolve), so it
/// can be created without a [`Cx`] and translated wherever it is used. In a
/// view, as content or as an attribute value, it renders as the text it
/// resolves to.
pub struct Translation<'a> {
    namespace: &'static str,
    id: &'static str,
    attribute: Option<&'static str>,
    args: FluentArgs<'a>,
}

impl<'a> Translation<'a> {
    /// Creates a translation of the message that `record`, a
    /// [`RawTranslation`], names, formatted with `args`.
    ///
    /// # Panics
    ///
    /// Panics when the record is invalid. The records that `t!` creates are
    /// always valid.
    #[doc(hidden)]
    #[must_use]
    pub fn new(record: &'static [u8], args: FluentArgs<'a>) -> Self {
        // Tools find `t!` calls by searching the binary for their records.
        // Reading through `black_box` keeps the optimizer from removing them.
        let record = hint::black_box(record);
        let (namespace, id, attribute) = RawTranslation::decode_key(&mut ConstReader::new(record))
            .expect("the translation record is valid");
        Self {
            namespace,
            id,
            attribute,
            args,
        }
    }

    /// Resolves the translation into its text for the [locale](try_locale) of
    /// `cx`, like [`resolve_in`](Self::resolve_in).
    ///
    /// When `cx` has no locale, the result is the placeholder described there.
    #[must_use]
    pub fn resolve(&self, cx: &Cx) -> Cow<'static, str> {
        match try_locale(cx) {
            Some(locale) => self.resolve_in(locale),
            None => Cow::Owned(self.placeholder()),
        }
    }

    /// Resolves the translation into its text for `locale`, as described in
    /// [`Catalog::format`](crate::Catalog::format).
    ///
    /// When the namespace has no text for `locale`, the result is a
    /// placeholder made of the key and the arguments, such as
    /// `password-field.placeholder(min = 8)`. String arguments are quoted, and
    /// numbers are shown without formatting.
    ///
    /// The text is borrowed from the catalog when the message has nothing to
    /// format, such as a message without variables.
    #[must_use]
    pub fn resolve_in(&self, locale: &Locale) -> Cow<'static, str> {
        Namespace::catalog(self.namespace)
            .and_then(|catalog| catalog.format(locale, self.id, self.attribute, Some(&self.args)))
            .unwrap_or_else(|| Cow::Owned(self.placeholder()))
    }

    /// Returns the key and the arguments of the translation, which stand in
    /// for a missing message.
    fn placeholder(&self) -> String {
        let mut text = String::from(self.id);
        if let Some(attribute) = self.attribute {
            text.push('.');
            text.push_str(attribute);
        }
        // The arguments are sorted by name.
        for (index, (name, value)) in self.args.iter().enumerate() {
            text.push_str(if index == 0 { "(" } else { ", " });
            text.push_str(name);
            text.push_str(" = ");
            let _ = match value {
                FluentValue::String(value) => write!(text, "{value:?}"),
                FluentValue::Number(number) => write!(text, "{}", number.value),
                value => write!(text, "{value:?}"),
            };
        }
        if self.args.iter().next().is_some() {
            text.push(')');
        }
        text
    }

    /// Appends the resolved text to the view being built, escaped.
    #[cfg(feature = "view")]
    fn push(&self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        match self.resolve(cx) {
            Cow::Borrowed(text) => parts.push_static_str(text),
            Cow::Owned(text) => parts.push_string(text),
        };
    }
}

/// A translation used in node position renders as the text it resolves to.
#[cfg(feature = "view")]
impl NodeViewParts for Translation<'_> {
    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push(cx, parts);
    }
}

/// A translation used in attribute value position renders as the text it
/// resolves to, e.g. `placeholder=(t!(login-input.placeholder))`.
#[cfg(feature = "view")]
impl AttributeValueViewParts for Translation<'_> {
    fn attribute_present(&self) -> bool {
        true
    }

    fn into_view_parts(self, cx: &Cx, parts: &mut PartsWriter<'_>) {
        self.push(cx, parts);
    }
}

#[cfg(test)]
mod tests {
    use http::{Request, header::ACCEPT_LANGUAGE};
    use topcoat::view::{View, ViewExt, view};

    use super::*;
    use crate::{NamespaceLocale, locale, test_namespace_record, test_translation_record};

    const NAMESPACE: &str = "translation";

    inventory::submit! {
        Namespace::new(
            test_namespace_record!(NAMESPACE),
            &[
                NamespaceLocale::new(
                    "en",
                    &["\
title = Welcome
greeting = Hello, { $name }!
login-input = Log in
    .placeholder = Your name
"],
                ),
                NamespaceLocale::new(
                    "de",
                    &["\
title = Willkommen
greeting = Hallo, { $name }!
"],
                ),
            ],
        )
    }

    fn title() -> Translation<'static> {
        Translation::new(
            test_translation_record!(NAMESPACE, "title", None),
            FluentArgs::new(),
        )
    }

    /// Returns a context for a request with the `Accept-Language` header
    /// `header`.
    fn request(header: &str) -> Cx {
        let (parts, ()) = Request::builder()
            .header(ACCEPT_LANGUAGE, header)
            .body(())
            .expect("request should build")
            .into_parts();
        Cx::default().with(parts)
    }

    #[test]
    fn resolves_for_the_locale_of_the_scope() {
        let cx = Cx::default().with(locale!("de"));
        assert_eq!(title().resolve(&cx), "Willkommen");
    }

    #[test]
    fn resolves_in_a_locale_other_than_the_one_of_the_scope() {
        let title = title();
        let cx = Cx::default().with(locale!("en"));
        assert_eq!(title.resolve(&cx), "Welcome");
        assert_eq!(title.resolve_in(&locale!("de")), "Willkommen");
        assert_eq!(title.resolve_in(&locale!("fr")), "title");
    }

    #[test]
    fn a_locale_the_catalog_lacks_resolves_to_the_placeholder_instead_of_using_the_header() {
        // The page is in French, so the German the header asks for would be
        // out of place.
        let cx = request("de").with(locale!("fr"));
        assert_eq!(title().resolve(&cx), "title");
    }

    #[test]
    fn resolves_to_the_placeholder_without_a_locale() {
        let cx = request("de");
        assert_eq!(title().resolve(&cx), "title");
    }

    #[test]
    fn resolves_to_the_placeholder_for_an_unknown_namespace() {
        let title = Translation::new(
            test_translation_record!("unknown", "title", None),
            FluentArgs::new(),
        );
        let cx = Cx::default().with(locale!("en"));
        assert_eq!(title.resolve(&cx), "title");
    }

    #[test]
    fn the_placeholder_shows_the_key_and_the_arguments_sorted_by_name() {
        let mut args = FluentArgs::new();
        args.set("name", "Ada");
        args.set("count", 8);
        let missing = Translation::new(
            test_translation_record!(NAMESPACE, "missing", Some("label")),
            args,
        );
        let cx = Cx::default().with(locale!("en"));
        assert_eq!(
            missing.resolve(&cx),
            r#"missing.label(count = 8, name = "Ada")"#
        );
    }

    #[test]
    fn formats_borrowed_arguments() {
        let name = String::from("Ada");
        let mut args = FluentArgs::new();
        args.set("name", name.as_str());
        let greeting =
            Translation::new(test_translation_record!(NAMESPACE, "greeting", None), args);
        let cx = Cx::default().with(locale!("de"));
        // Fluent isolates the argument from the surrounding text.
        assert_eq!(greeting.resolve(&cx), "Hallo, \u{2068}Ada\u{2069}!");
    }

    #[test]
    fn borrows_text_without_arguments() {
        let cx = Cx::default().with(locale!("en"));
        assert!(matches!(title().resolve(&cx), Cow::Borrowed(_)));
    }

    async fn render(view: impl View, cx: &Cx) -> String {
        view.single().await.unwrap().render(cx)
    }

    #[tokio::test]
    async fn renders_as_escaped_content() {
        let mut args = FluentArgs::new();
        args.set("name", "<b>");
        let greeting =
            Translation::new(test_translation_record!(NAMESPACE, "greeting", None), args);
        let cx = &Cx::default().with(locale!("en"));
        let html = render(view! { cx => <p>(greeting)</p> }, cx).await;
        assert_eq!(html, "<p>Hello, \u{2068}&lt;b&gt;\u{2069}!</p>");
    }

    #[tokio::test]
    async fn renders_as_an_attribute_value() {
        let placeholder = Translation::new(
            test_translation_record!(NAMESPACE, "login-input", Some("placeholder")),
            FluentArgs::new(),
        );
        let cx = &Cx::default().with(locale!("en"));
        let html = render(view! { cx => <input placeholder=(placeholder)> }, cx).await;
        assert_eq!(html, r#"<input placeholder="Your name">"#);
    }
}
