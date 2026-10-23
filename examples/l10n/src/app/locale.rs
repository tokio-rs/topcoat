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

// The locale is the first part of the URL, as in /en or /de.
module_param!(pub path_locale: Locale, error = not_found);

// Sets the locale for every page in this module. Each `t!` translates into it.
#[layer]
async fn layer(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let path_locale = path_param::<PathLocale>(cx)?;
    // Any locale can be typed into the URL, so reject the ones without messages.
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
        // Set `lang` and `dir` to match the locale.
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
        <nav>
            for link_locale in t::locales() {
                <a
                    href=(href!(_page::page, PathLocale(link_locale.clone())))
                    lang=(link_locale.id())
                    if link_locale == locale(cx) {
                        aria-current="page"
                    }
                >
                    // Names each language in itself, like "Deutsch", by translating into
                    // the link's locale instead of the page's.
                    (t!(language).resolve(&cx.with(link_locale.clone())))
                </a>
                " "
            }
        </nav>
    })
}
