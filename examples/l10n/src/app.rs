mod locale;

use topcoat::{
    Result,
    context::Cx,
    l10n::{accept_language, fluent::messages, locale, negotiate},
    router::{Router, error::redirect, href, module_router, page},
    view::View,
};

// The path is relative to this file, so the messages live in `src/l10n/`.
messages!("./l10n/{locale}.ftl");

// The `module_router!()` macro call must be placed at the root of your route structure.
// In this case, the `app` module is marked as the root.
pub fn router() -> Router {
    module_router!().build()
}

// The page at / sends the visitor to the pages in the locale that best matches the
// `Accept-Language` header, or English when none matches.
#[page]
async fn redirect_to_locale(cx: &Cx) -> Result<impl View> {
    let locale = negotiate(accept_language(cx), t::locales())
        .cloned()
        .unwrap_or(locale!("en"));
    Err::<(), _>(
        redirect(href!(locale::_page::page, locale::PathLocale(locale)).resolve(cx)).into(),
    )
}
