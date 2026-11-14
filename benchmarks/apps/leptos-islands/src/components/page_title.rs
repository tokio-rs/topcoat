use leptos::prelude::*;
use leptos_meta::Title;

/// Sets the document title to `<title> | Harbor Market`, or just the store name without a title.
#[component]
pub fn PageTitle(#[prop(optional, into)] title: Option<String>) -> impl IntoView {
    let text = match title {
        Some(title) => format!("{title} | Harbor Market"),
        None => "Harbor Market".to_owned(),
    };
    view! { <Title text /> }
}
