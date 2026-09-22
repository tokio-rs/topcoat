use std::time::Duration;

use topcoat::{
    Result,
    router::page,
    view::{View, emit, live, view},
};

// The browser receives the shell with the placeholder right away, and the
// content replaces it in place once the lookup finishes.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"Suspense"</h1>
        (live! {
            emit! { <p>"Loading..."</p> }?;
            let content = load_content().await;
            emit! { <p>(content)</p> }
        })
    })
}

// Stands in for a slow database query or upstream request.
async fn load_content() -> &'static str {
    tokio::time::sleep(Duration::from_secs(2)).await;
    "This content took two seconds to load."
}
