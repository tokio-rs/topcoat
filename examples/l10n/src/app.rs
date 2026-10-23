mod locale;

use topcoat::{
    Result,
    context::Cx,
    l10n::{accept_language, fluent::messages, locale, negotiate},
    router::{Router, error::redirect, href, module_router, page},
    view::View,
};

// Loads `src/l10n/en.ftl`, `src/l10n/de.ftl` and so on, and creates `t!` to translate them.
messages!("./l10n/{locale}.ftl");

// The `module_router!()` macro call must be placed at the root of your route structure.
// In this case, the `app` module is marked as the root.
pub fn router() -> Router {
    module_router!().build()
}

// Sends the visitor from / to the locale that best fits the languages of their browser.
#[page]
async fn redirect_to_locale(cx: &Cx) -> Result<impl View> {
    let locale = negotiate(accept_language(cx), t::locales())
        .cloned()
        .unwrap_or(locale!("en"));
    Err::<(), _>(
        redirect(href!(locale::_page::page, locale::PathLocale(locale)).resolve(cx)).into(),
    )
}
