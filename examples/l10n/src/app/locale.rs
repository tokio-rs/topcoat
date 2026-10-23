pub mod _page;

use topcoat::{
    Result,
    context::Cx,
    l10n::{Locale, locale},
    router::{
        Body, Next, Slot, error::not_found, layer, layout, module_param, path_param,
        response::Response,
    },
    view::{View, view},
};

use super::t;

module_param!(pub path_locale: Locale, error = not_found);

#[layer]
async fn layer(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let path_locale = path_param::<PathLocale>(cx)?;
    if !t::locales().contains(path_locale) {
        return Err(not_found().into());
    }
    let cx = cx.with(path_locale.clone());
    next.run(&cx, body).await
}

#[layout]
async fn layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang=(locale(cx).id()) dir=(locale(cx).id().direction())>
            <head>
                <title>(t!(title))</title>
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}
