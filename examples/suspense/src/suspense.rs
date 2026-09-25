use std::time::Duration;

use topcoat::{
    Result,
    router::page,
    view::{View, component, suspense, view},
};

// The shell and the fallback reach the browser right away; the content
// replaces the fallback in place once it has rendered.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"Suspense"</h1>
        suspense(fallback: view! { <p>"Loading..."</p> }, content())
    })
}

#[component]
async fn content() -> Result<impl View> {
    Ok(view! { <p>(load_content().await)</p> })
}

// Stands in for a slow database query or upstream request.
async fn load_content() -> &'static str {
    tokio::time::sleep(Duration::from_secs(2)).await;
    "This content took two seconds to load."
}
