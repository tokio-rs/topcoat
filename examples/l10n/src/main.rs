mod locale;

use topcoat::{
    Result, context::Cx, l10n::{fluent::messages, locale}, router::{error::redirect, href, module_router, page}, view::View,
};

messages!("./l10n/{locale}.ftl");

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

#[page]
async fn redirect_to_locale(cx: &Cx) -> Result<impl View> {
    Err::<(), _>(
        redirect(href!(locale::page, locale::LocaleParam(locale!("en"))).resolve(cx)).into(),
    )
}
