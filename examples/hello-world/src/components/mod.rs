use topcoat::{
    context::{Cx, app_state},
    router::{Result, uri},
    view::{View, component, view},
};

#[component]
async fn button(id: &str, child: View) -> Result {
    view! { <button id=(id) class="button">(child)</button> }
}

#[component]
pub async fn app_and_request_state(cx: &Cx) -> Result {
    view! {
        "current page: "

        (uri(cx).to_string())

        ", app state: "

        (app_state::<i32>(cx))
    }
}
