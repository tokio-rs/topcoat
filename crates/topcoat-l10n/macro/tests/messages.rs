use topcoat::{
    Result,
    context::Cx,
    l10n::{
        Locale, RouterBuilderL10nExt,
        fluent::{messages, t},
        locale, try_locale,
    },
    router::{Body, Next, Router, layer, page, response::Response, to_bytes},
    view::{View, ViewExt, view},
};

// The files are found at the default path, `tests/l10n/{locale}.ftl`.
messages!(fallback = "en");

mod admin {
    topcoat::l10n::fluent::messages!(pub "./admin/{locale}.ftl");
}

/// Returns a context whose locale is `locale`.
fn scope(locale: Locale) -> Cx {
    Cx::default().with(locale)
}

#[test]
fn translates_into_the_locale_of_the_scope() {
    assert_eq!(t!(msg::title).resolve(&scope(locale!("de"))), "Willkommen");
    assert_eq!(t!(msg::title).resolve(&scope(locale!("en"))), "Welcome");
}

#[test]
fn takes_what_a_locale_lacks_from_the_fallback() {
    let de = scope(locale!("de"));
    assert_eq!(t!(msg::only_english).resolve(&de), "Only in English");
    // German has the message, but not the attribute.
    let user_name = "Ada";
    assert_eq!(
        t!(msg::login_input.title, user_name).resolve(&de),
        "Log in as \u{2068}Ada\u{2069}"
    );
    // A locale without files takes every message from the fallback.
    assert_eq!(t!(msg::title).resolve(&scope(locale!("fr"))), "Welcome");
}

#[test]
fn formats_numbers_and_plurals_for_the_locale() {
    let de = scope(locale!("de"));
    assert_eq!(t!(msg::items, count = 1).resolve(&de), "Ein Artikel");
    assert_eq!(
        t!(msg::items, count = 1234).resolve(&de),
        "\u{2068}1.234\u{2069} Artikel"
    );
    assert_eq!(
        t!(msg::items, count = 1234).resolve(&scope(locale!("en"))),
        "\u{2068}1,234\u{2069} items"
    );
}

#[test]
fn passes_arguments_by_name_or_shorthand() {
    let en = scope(locale!("en"));
    let user_name = String::from("Ada");
    assert_eq!(
        t!(msg::greeting, user_name).resolve(&en),
        "Hello, \u{2068}Ada\u{2069}!"
    );
    assert_eq!(
        t!(msg::greeting, user_name = "Bob").resolve(&en),
        "Hello, \u{2068}Bob\u{2069}!"
    );
}

#[test]
fn a_referenced_message_shares_the_arguments() {
    let en = scope(locale!("en"));
    assert_eq!(
        t!(msg::welcome, user_name = "Ada", count = 2).resolve(&en),
        "Hello, \u{2068}Ada\u{2069}! You have \u{2068}2\u{2069} items."
    );
}

#[test]
fn a_keyword_becomes_a_raw_identifier() {
    let en = scope(locale!("en"));
    assert_eq!(
        t!(msg::r#type, r#type = "raw").resolve(&en),
        "Type: \u{2068}raw\u{2069}"
    );
}

#[test]
fn each_catalog_has_its_own_messages_and_locales() {
    let de = scope(locale!("de"));
    assert_eq!(t!(msg::title).resolve(&de), "Willkommen");
    assert_eq!(t!(admin::msg::title).resolve(&de), "Administration");
    assert_eq!(
        msg::CATALOG.supported_locales().locales(),
        [locale!("de"), locale!("en")]
    );
    assert_eq!(
        admin::msg::CATALOG.supported_locales().locales(),
        [locale!("en")]
    );
}

#[tokio::test]
async fn renders_as_content_and_attribute_values() {
    let cx = &scope(locale!("de"));
    let user_name = "<Ada>";
    let view = view! { cx =>
        <p>(t!(msg::greeting, user_name))</p>
        <input placeholder=(t!(msg::login_input.placeholder))>
    };
    assert_eq!(
        view.single().await.unwrap().render(cx),
        "<p>Hallo, \u{2068}&lt;Ada&gt;\u{2069}!</p><input placeholder=\"Dein Name\">"
    );
}

/// Shows the locale of the page, if any, and a translated title.
#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let lang = try_locale(cx).map(ToString::to_string);
    Ok(view! { <p lang=(lang)>(t!(msg::title))</p> })
}

/// Makes the locales of the catalog the locales of every page.
#[layer("/")]
async fn catalog_locales(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let cx = cx.with(msg::CATALOG.supported_locales().clone());
    next.run(&cx, body).await
}

/// Requests the home page with the `Accept-Language` header `header`.
async fn get(router: &Router, header: &str) -> String {
    let request = http::Request::builder()
        .uri("/")
        .header(http::header::ACCEPT_LANGUAGE, header)
        .body(Body::empty())
        .unwrap();
    let response = router.handle(request).await;
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

#[tokio::test]
async fn translates_for_the_header_when_the_page_has_no_locale() {
    let router = Router::builder().page(home).build();
    assert_eq!(get(&router, "fr, de;q=0.5").await, "<p>Willkommen</p>");
}

#[tokio::test]
async fn a_layer_can_give_pages_the_locales_of_a_catalog() {
    let router = Router::builder().page(home).layer(catalog_locales).build();
    assert_eq!(
        get(&router, "de-AT").await,
        r#"<p lang="de">Willkommen</p>"#
    );
    assert_eq!(get(&router, "fr").await, r#"<p lang="en">Welcome</p>"#);
}

#[tokio::test]
async fn the_page_locale_wins_over_the_header() {
    // French is supported, but the catalog has no French, so the page gets
    // the fallback rather than the German the header also accepts.
    let router = Router::builder()
        .page(home)
        .supported_locales([locale!("fr"), locale!("de")])
        .build();
    assert_eq!(get(&router, "fr, de").await, r#"<p lang="fr">Welcome</p>"#);
    assert_eq!(get(&router, "de").await, r#"<p lang="de">Willkommen</p>"#);
}
