mod locale;

use topcoat::{
    Result,
    context::Cx,
    l10n::langid,
    router::{error::redirect, href, module_router, page},
    view::View,
};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

#[page]
async fn redirect_to_locale(cx: &Cx) -> Result<impl View> {
    Err::<(), _>(redirect(href!(locale::page, locale::Locale(langid!("en"))).resolve(cx)).into())
}
