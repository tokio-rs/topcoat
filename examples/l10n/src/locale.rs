use topcoat::{
    Result,
    context::Cx,
    l10n::{Locale, locale},
    router::{Slot, layout, module_param, page, path_param},
    view::{View, view},
};

use crate::t;

module_param!(pub locale_param: Locale, error = not_found);

#[layout]
async fn layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    if !catalog
        .supported_locales()
        .contains(path_param::<LocaleParam>())
    {
        return Err(not_found().into());
    }

    let cx = cx.with(path_param::<LocaleParam>());

    Ok(view! {
        cx =>
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

#[page("./{locale}")]
pub async fn page() -> Result<impl View> {
    Ok(view! { (t!(current_date)) })
}
