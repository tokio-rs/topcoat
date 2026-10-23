use std::borrow::Cow;

use fluent_bundle::FluentArgs;
use topcoat_core::context::Cx;
#[cfg(feature = "view")]
use topcoat_view::{AttributeValueViewParts, NodeViewParts, PartsWriter};

use crate::{Catalog, try_locale};

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

    /// Resolves the translation into its text for the [locale](try_locale) of
    /// `cx`, as described in [`Catalog::format`]. When `cx` has no locale, the
    /// result is the message ID.
    ///
    /// The text is borrowed from the catalog when the message has nothing to
    /// format, such as a message without variables.
    #[must_use]
    pub fn resolve(&self, cx: &Cx) -> Cow<'static, str> {
        // TODO: Render the placeholder `key.attr(args)` instead of the ID.
        match try_locale(cx) {
            Some(locale) => self
                .catalog
                .format(locale, self.id, self.attribute, Some(&self.args)),
            None => Cow::Borrowed(self.id),
        }
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
        Catalog::new([
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
        ])
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
    fn a_locale_the_catalog_lacks_resolves_to_the_id_instead_of_using_the_header() {
        // The page is in French, so the German the header asks for would be
        // out of place.
        let cx = request("de").with(locale!("fr"));
        assert_eq!(title().resolve(&cx), "title");
    }

    #[test]
    fn resolves_to_the_id_without_a_locale() {
        let cx = request("de");
        assert_eq!(title().resolve(&cx), "title");
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
        let greeting = Translation::new(&CATALOG, "greeting", None, args);
        let cx = &Cx::default().with(locale!("en"));
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
        let cx = &Cx::default().with(locale!("en"));
        let html = render(view! { cx => <input placeholder=(placeholder)> }, cx).await;
        assert_eq!(html, r#"<input placeholder="Your name">"#);
    }
}
