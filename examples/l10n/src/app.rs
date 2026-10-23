mod locale;

use topcoat::{
    Result,
    context::Cx,
    l10n::{accept_language, fluent::messages, locale, negotiate},
    router::{Router, error::redirect, href, module_router, page},
    view::View,
};

// Loads one Fluent file per language, such as `src/l10n/en.ftl` and `src/l10n/de.ftl`.
// `{locale}` stands for the language code. This also creates the `t!` macro, which looks up
// a message in the visitor's language.
messages!("./l10n/{locale}.ftl");

// The `module_router!()` macro call must be placed at the root of your route structure.
// In this case, the `app` module is marked as the root.
pub fn router() -> Router {
    module_router!().build()
}

// The page at / picks a language and sends the visitor to it, for example to /de.
// Browsers send the languages the visitor prefers in the `Accept-Language` header.
// `negotiate` picks the best one that we have messages for. If none match, we use English.
#[page]
async fn redirect_to_locale(cx: &Cx) -> Result<impl View> {
    let locale = negotiate(accept_language(cx), t::locales())
        .cloned()
        .unwrap_or(locale!("en"));
    Err::<(), _>(
        redirect(href!(locale::_page::page, locale::PathLocale(locale)).resolve(cx)).into(),
    )
}
