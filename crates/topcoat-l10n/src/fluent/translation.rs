use std::{borrow::Cow, slice};

use fluent_bundle::FluentArgs;
use topcoat_core::context::Cx;
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Catalog, accept_language, try_locale};

/// A message of a [`Catalog`], or one of its attributes, with its arguments.
///
/// A translation is not formatted until it is [resolved](Self::resolve), so it
/// can be created without a [`Cx`] and translated wherever it is used. In a
/// view, as content or as an attribute value, it renders as the text it
/// resolves to.
pub struct Translation<'a> {
    catalog: &'static Catalog,
    id: &'static str,
    attribute: Option<&'static str>,
    args: FluentArgs<'a>,
}

impl<'a> Translation<'a> {
    /// Creates a translation of the message `id` of `catalog`, or of its
    /// `attribute`, formatted with `args`.
    #[must_use]
    pub const fn new(
        catalog: &'static Catalog,
        id: &'static str,
        attribute: Option<&'static str>,
        args: FluentArgs<'a>,
    ) -> Self {
        Self {
            catalog,
            id,
            attribute,
            args,
        }
    }

    /// Resolves the translation into its text for the locale of `cx`.
    ///
    /// When `cx` has a [locale](try_locale), the message is translated into
    /// it. Otherwise, it is translated into the locale that best matches the
    /// request's [`Accept-Language`](accept_language) header. Either way, a
    /// locale the catalog lacks, or a message the locale lacks, falls back as
    /// described in [`Catalog::format`].
    ///
    /// The text is borrowed from the catalog when the message has nothing to
    /// format, such as a message without variables.
    #[must_use]
    pub fn resolve(&self, cx: &Cx) -> Cow<'static, str> {
        let requested = match try_locale(cx) {
            Some(locale) => slice::from_ref(locale),
            None => accept_language(cx),
        };
        self.catalog
            .format(requested, self.id, self.attribute, Some(&self.args))
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
/// resolves to, e.g. `placeholder=(t!(msg::login_input::placeholder))`.
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
    use std::sync::LazyLock;

    use http::{Request, header::ACCEPT_LANGUAGE};
    use topcoat::view::{View, ViewExt, view};

    use super::*;
    use crate::locale;

    static CATALOG: LazyLock<Catalog> = LazyLock::new(|| {
        Catalog::new(
            locale!("en"),
            [
                (
                    locale!("en"),
                    &["\
title = Welcome
greeting = Hello, { $name }!
login-input = Log in
    .placeholder = Your name
"][..],
                ),
                (
                    locale!("de"),
                    &["\
title = Willkommen
greeting = Hallo, { $name }!
"][..],
                ),
            ],
        )
    });

    fn title() -> Translation<'static> {
        Translation::new(&CATALOG, "title", None, FluentArgs::new())
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
    fn a_locale_the_catalog_lacks_falls_back_instead_of_using_the_header() {
        // The page is in French, so the German the header asks for would be
        // out of place.
        let cx = request("de").with(locale!("fr"));
        assert_eq!(title().resolve(&cx), "Welcome");
    }

    #[test]
    fn resolves_for_the_header_without_a_locale() {
        let cx = request("fr, de;q=0.5");
        assert_eq!(title().resolve(&cx), "Willkommen");
    }

    #[test]
    fn resolves_to_the_fallback_outside_a_request() {
        assert_eq!(title().resolve(&Cx::default()), "Welcome");
    }

    #[test]
    fn formats_borrowed_arguments() {
        let name = String::from("Ada");
        let mut args = FluentArgs::new();
        args.set("name", name.as_str());
        let greeting = Translation::new(&CATALOG, "greeting", None, args);
        let cx = Cx::default().with(locale!("de"));
        // Fluent isolates the argument from the surrounding text.
        assert_eq!(greeting.resolve(&cx), "Hallo, \u{2068}Ada\u{2069}!");
    }

    #[test]
    fn borrows_text_without_arguments() {
        assert!(matches!(title().resolve(&Cx::default()), Cow::Borrowed(_)));
    }

    async fn render(view: impl View, cx: &Cx) -> String {
        view.single().await.unwrap().render(cx)
    }

    #[tokio::test]
    async fn renders_as_escaped_content() {
        let mut args = FluentArgs::new();
        args.set("name", "<b>");
        let greeting = Translation::new(&CATALOG, "greeting", None, args);
        let cx = &Cx::default();
        let html = render(view! { cx => <p>(greeting)</p> }, cx).await;
        assert_eq!(html, "<p>Hello, \u{2068}&lt;b&gt;\u{2069}!</p>");
    }

    #[tokio::test]
    async fn renders_as_an_attribute_value() {
        let placeholder = Translation::new(
            &CATALOG,
            "login-input",
            Some("placeholder"),
            FluentArgs::new(),
        );
        let cx = &Cx::default();
        let html = render(view! { cx => <input placeholder=(placeholder)> }, cx).await;
        assert_eq!(html, r#"<input placeholder="Your name">"#);
    }
}
