use std::time::Duration;

use topcoat::{
    Result,
    router::page,
    view::{View, component, error_boundary, suspense, view},
};

// Each section streams in behind its own fallback. The first lookup fails,
// but the boundary around it turns the error into a message in place, and
// the section next to it is unaffected.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"Error boundary"</h1>
        error_boundary(
            fallback: |error| Ok(
                    view! {
                        <p>
                            "The first section is unavailable: "
                            (error.to_string())
                        </p>
                    },
                ),
            suspense(fallback: view! { <p>"Loading the first section..."</p> }, failing())
        )
        suspense(fallback: view! { <p>"Loading the second section..."</p> }, succeeding())
    })
}

#[component]
async fn failing() -> Result<impl View> {
    let content = load_content().await?;
    Ok(view! { <p>(content)</p> })
}

// Stands in for an upstream service that is down.
async fn load_content() -> Result<&'static str> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    Err(std::io::Error::other("the upstream service is unreachable").into())
}

#[component]
async fn succeeding() -> Result<impl View> {
    tokio::time::sleep(Duration::from_secs(2)).await;
    Ok(view! { <p>"The second section loaded on its own."</p> })
}
