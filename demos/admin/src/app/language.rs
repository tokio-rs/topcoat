//! Lets visitors choose the language of the app.

use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    icon::{icon, iconify::iconify_icon},
    l10n::{Locale, fluent::messages, locale},
    router::{
        content::Form,
        error::{SeeOther, bad_request},
        href,
        request::uri,
        route,
    },
    view::{View, attributes, component, element_id, view},
};

use super::_common::see_back;
use crate::{
    components::{
        button::{ButtonSize, ButtonVariant, button_variants},
        dropdown_menu::{
            dropdown_menu, dropdown_menu_content, dropdown_menu_item, dropdown_menu_trigger,
        },
        floating::FloatingAlign,
    },
    features::locale::{LOCALES, choose_locale},
};

messages!("{mod}/l10n/{locale}.ftl");

#[derive(Deserialize)]
pub struct Choice {
    locale: Locale,
    return_to: String,
}

#[route(POST)]
pub async fn choose(cx: &Cx, Form(form): Form<Choice>) -> Result<SeeOther> {
    if !LOCALES.contains(&form.locale) {
        return Err(bad_request("unsupported language").into());
    }
    choose_locale(cx, form.locale)?;
    Ok(see_back(cx, &form.return_to))
}

/// A button that opens a menu of the app's languages and switches to the one picked.
#[component]
pub async fn language_menu(cx: &Cx) -> Result<impl View> {
    let menu = element_id(cx);
    let current = locale(cx);
    let return_to = uri(cx).path_and_query().map_or("/", |path| path.as_str());

    Ok(view! {
        dropdown_menu(
            dropdown_menu_trigger(
                target: &menu,
                attrs: attributes! {
                    class=(button_variants(ButtonVariant::Ghost, ButtonSize::Icon))
                    aria-label=(t!(change_language))
                    title=(t!(change_language))
                },
                icon(data: iconify_icon!("lucide:languages"))
            )
            dropdown_menu_content(
                id: &menu,
                align: FloatingAlign::End,
                <form method="post" action=(href!(choose))>
                    <input type="hidden" name="return_to" value=(return_to)>
                    for option in &LOCALES {
                        dropdown_menu_item(
                            attrs: attributes! {
                                type="submit"
                                name="locale"
                                value=(option.to_string())
                                lang=(option.id())
                                aria-current=((option == current).then_some("true"))
                            },
                            // Names each language in itself, such as "Deutsch".
                            (t!(language).resolve_in(option))
                            if option == current {
                                icon(
                                    data: iconify_icon!("lucide:check"),
                                    attrs: attributes! { class="ml-auto" }
                                )
                            }
                        )
                    }
                </form>
            )
        )
    })
}
