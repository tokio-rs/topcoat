use topcoat::{component, router::Result, view, view::View};

#[component]
async fn button(cx: Cx<'_>, id: &str, child: View) -> Result {
    view! { <button id=(id) class="button">(child)</button> }
}
