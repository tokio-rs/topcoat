mod _admin;
mod _common;
mod language;
pub mod login;
mod logout;
mod theme;

use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::Cx,
    cookie::RouterBuilderCookieExt,
    font::{Font, fontsource::fontsource_font},
    l10n::locale,
    router::{
        Body, Next, RouterBuilder, RouterBuilderDiscoverExt, Slot, layer, layout, module_router,
        response::Response,
    },
    runtime::RouterBuilderRuntimeExt,
    session::{RouterBuilderSessionExt, SessionConfig},
    tailwind,
    view::{View, view},
};

use crate::features::{locale::preferred_locale, theme::theme};

const GEIST: Font =
    fontsource_font!(GEIST, weight: [400, 500, 600, 700], style: Normal, host: Asset);

pub fn router() -> RouterBuilder {
    module_router!()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .cookies()
        .sessions(SessionConfig::default())
        // Register the runtime after the application layers so rerenders use them too.
        .runtime()
}

/// Sets the language for every request, including shard updates.
#[layer]
async fn set_locale(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let locale = preferred_locale(cx).clone();
    next.run(&cx.with(locale), body).await
}

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let theme = theme(cx);

    Ok(view! {
        <!DOCTYPE html>
        <html
            lang=(locale(cx).id())
            dir=(locale(cx).id().direction())
            class=(theme.class())
            style=(theme.color_scheme())
        >
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>"Backroom"</title>
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                topcoat::font::link(font: GEIST)
                topcoat::runtime::script()
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}
