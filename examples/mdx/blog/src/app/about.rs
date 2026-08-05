use topcoat::{Result, router::page, view::view};

#[page]
async fn about() -> Result {
    view! { <h1>"About"</h1> }
}
