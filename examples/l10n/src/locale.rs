use topcoat::{
    Result,
    l10n::{
        LanguageIdentifier,
        fluent::{messages, t},
    },
    router::{Slot, layout, module_param, page},
    view::{View, view},
};

module_param!(pub locale: LanguageIdentifier, error = not_found);

messages!("./l10n/{locale}.ftl");

#[layout]
async fn layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>(t!(msg::title))</title>
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}

#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! { "hi" })
}
