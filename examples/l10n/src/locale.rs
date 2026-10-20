use topcoat::{
    Result,
    l10n::{LanguageIdentifier, fluent::t},
    router::{Slot, layout, module_param, page},
    view::{View, view},
};

use crate::msg;

module_param!(pub locale: LanguageIdentifier, error = not_found);

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
    Ok(view! { (t!(msg::current_date)) })
}
