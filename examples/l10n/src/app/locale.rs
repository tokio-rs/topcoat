pub mod _page;

use topcoat::{
    Result,
    context::Cx,
    l10n::{Locale, locale},
    router::{
        Body, Next, Slot, error::not_found, href, layer, layout, module_param, path_param,
        response::Response,
    },
    view::{View, component, view},
};

use super::t;

// The locale comes from the URL, as in /en or /de.
module_param!(pub path_locale: Locale, error = not_found);

// Only accept locales that have messages, then put the locale in `cx`. `t!` reads it from
// there to pick the translation.
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
        // The `lang` and `dir` attributes come from the locale.
        <html lang=(locale(cx).id()) dir=(locale(cx).id().direction())>
            <head>
                <title>(t!(title))</title>
                topcoat::dev::script()
            </head>
            <body>
                nav()

                (slot)
            </body>
        </html>
    })
}

/// Links to the page in every locale that has messages.
#[component]
async fn nav(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // Each link shows its language's own name, like "Deutsch", so it is translated
        // into the link's locale instead of the page's.
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
    })
}
