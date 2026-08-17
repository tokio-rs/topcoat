use topcoat::{Result, router::page, view::view};

// _marketing is skipped in the URL, so this page is /pricing.
#[page]
pub async fn page() -> Result {
    view! {
        <h1>"pricing"</h1>
        <p>"src/app/_marketing/pricing.rs -> /pricing"</p>
    }
}
