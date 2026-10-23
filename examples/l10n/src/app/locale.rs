pub mod _page;

use topcoat::{
    Result,
    context::Cx,
    l10n::{Locale, locale},
    router::{
        Body, Next, Slot, error::not_found, href, layer, layout, module_param, path_param,
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
            <body>
                // Each link names its language in that language, so it is resolved with
                // the link's locale instead of the page's.
                <nav>
                    for link_locale in t::locales() {
                        <a
                            href=(href!(_page::page, PathLocale(link_locale.clone())))
                            lang=(link_locale.id())
                            if link_locale == locale(cx) {
                                aria-current="page"
                            }
                        >
                            (t!(language).resolve(&cx.with(link_locale.clone())))
                        </a>
                        " "
                    }
                </nav>
                (slot)
            </body>
        </html>
    })
}
