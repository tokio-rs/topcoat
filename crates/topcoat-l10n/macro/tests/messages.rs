use topcoat::{
    Result,
    context::Cx,
    l10n::{Locale, accept_language, fluent::messages, locale, negotiate, try_locale},
    router::{Body, Next, Router, layer, page, response::Response, to_bytes},
    view::{View, ViewExt, view},
};

messages!("./l10n/{locale}.ftl");

mod admin {
    topcoat::l10n::fluent::messages!(pub "./admin/{locale}.ftl");
}

/// Creates a context with the given locale.
fn scope(locale: Locale) -> Cx {
    Cx::new().with(locale)
}

#[test]
fn translates_into_the_locale_of_the_scope() {
    assert_eq!(t!(title).resolve(&scope(locale!("de"))), "Willkommen");
    assert_eq!(t!(title).resolve(&scope(locale!("en"))), "Welcome");
}

#[test]
fn a_regional_locale_uses_the_files_of_its_language() {
    assert_eq!(t!(title).resolve(&scope(locale!("de-AT"))), "Willkommen");
}

#[test]
fn a_missing_message_renders_the_placeholder() {
    let de = scope(locale!("de"));
    assert_eq!(t!(only - english).resolve(&de), "only-english");
    // German has the message, but not the attribute.
    assert_eq!(
        t!(login-input.title, user-name: "Ada").resolve(&de),
        r#"login-input.title(user-name: "Ada")"#
    );
    // A locale without files has no messages at all.
    assert_eq!(t!(title).resolve(&scope(locale!("fr"))), "title");
    // Neither has a scope without a locale.
    assert_eq!(t!(title).resolve(&Cx::new()), "title");
}

#[test]
fn formats_numbers_and_plurals_for_the_locale() {
    let de = scope(locale!("de"));
    assert_eq!(t!(items, count: 1).resolve(&de), "Ein Artikel");
    assert_eq!(
        t!(items, count: 1234).resolve(&de),
        "\u{2068}1.234\u{2069} Artikel"
    );
    assert_eq!(
        t!(items, count: 1234).resolve(&scope(locale!("en"))),
        "\u{2068}1,234\u{2069} items"
    );
}

#[test]
fn passes_arguments_by_name_or_shorthand() {
    let en = scope(locale!("en"));
    let name = String::from("Ada");
    assert_eq!(
        t!(greeting, name: "Bob").resolve(&en),
        "Hello, \u{2068}Bob\u{2069}!"
    );
    assert_eq!(
        t!(login-input.title, user-name: &name).resolve(&en),
        "Log in as \u{2068}Ada\u{2069}"
    );
    assert_eq!(
        t!(greeting, name).resolve(&en),
        "Hello, \u{2068}Ada\u{2069}!"
    );
}

#[test]
fn the_values_do_not_see_the_locals_of_the_expansion() {
    let en = scope(locale!("en"));
    let args = [1, 2];
    assert_eq!(
        t!(items, count: args.len()).resolve(&en),
        "\u{2068}2\u{2069} items"
    );
}

#[test]
fn a_referenced_message_shares_the_arguments() {
    let en = scope(locale!("en"));
    assert_eq!(
        t!(welcome, name: "Ada", count: 2).resolve(&en),
        "Hello, \u{2068}Ada\u{2069}! You have \u{2068}2\u{2069} items."
    );
}

#[test]
fn keywords_can_be_keys_and_arguments() {
    let en = scope(locale!("en"));
    assert_eq!(
        t!(type, r#type: "raw").resolve(&en),
        "Type: \u{2068}raw\u{2069}"
    );
    let r#type = "shorthand";
    assert_eq!(
        t!(type, r#type).resolve(&en),
        "Type: \u{2068}shorthand\u{2069}"
    );
}

#[test]
fn a_key_can_be_a_string() {
    assert_eq!(
        t!("login-input.placeholder").resolve(&scope(locale!("de"))),
        "Dein Name"
    );
}

#[test]
fn each_namespace_lists_the_locales_that_have_files() {
    assert_eq!(t::locales(), [locale!("de"), locale!("en")]);
    assert_eq!(admin::t::locales(), [locale!("en")]);
}

#[test]
fn a_binding_named_t_still_works_next_to_t() {
    let t = t!(title);
    assert_eq!(t.resolve(&scope(locale!("en"))), "Welcome");
}

#[test]
fn each_namespace_has_its_own_messages() {
    let de = scope(locale!("de"));
    assert_eq!(t!(title).resolve(&de), "Willkommen");
    // The admin namespace only has English.
    assert_eq!(admin::t!(title).resolve(&de), "title");
    assert_eq!(
        admin::t!(title).resolve(&scope(locale!("en"))),
        "Administration"
    );
}

#[tokio::test]
async fn renders_as_content_and_attribute_values() {
    let cx = &scope(locale!("de"));
    let name = "<Ada>";
    let view = view! {
        cx =>
        <p>(t!(greeting, name))</p>
        <input placeholder=(t!(login-input.placeholder))>
    };
    assert_eq!(
        view.single().await.unwrap().render(cx),
        "<p>Hallo, \u{2068}&lt;Ada&gt;\u{2069}!</p><input placeholder=\"Dein Name\">"
    );
}

/// Renders the page's locale, when set, and a translated title.
#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    let lang = try_locale(cx).map(ToString::to_string);
    Ok(view! { <p lang=(lang)>(t!(title))</p> })
}

/// Sets the page locale from `Accept-Language`, falling back to English if no locale
/// matches.
#[layer("/")]
async fn negotiate_locale(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let locale = negotiate(accept_language(cx), t::locales())
        .cloned()
        .unwrap_or(locale!("en"));
    next.run(&cx.with(locale), body).await
}

/// Sends a home page request with the given `Accept-Language` header.
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
async fn the_header_alone_does_not_choose_the_locale() {
    let router = Router::builder().page(home).build();
    assert_eq!(get(&router, "de").await, "<p>title</p>");
}

#[tokio::test]
async fn a_layer_chooses_the_locale_of_the_page() {
    let router = Router::builder().page(home).layer(negotiate_locale).build();
    assert_eq!(
        get(&router, "fr, de-AT;q=0.5").await,
        r#"<p lang="de">Willkommen</p>"#
    );
    assert_eq!(get(&router, "fr").await, r#"<p lang="en">Welcome</p>"#);
}
