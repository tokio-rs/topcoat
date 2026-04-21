use topcoat::{component, view, view::View};

#[component]
async fn button<'a>(id: &'a str, child: View) -> View {
    view! { <button id=(id) class="button">(child)</button> }
}
