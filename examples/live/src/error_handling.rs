use std::time::Duration;

use topcoat::{
    Result,
    router::page,
    view::{View, component, emit, live, view},
};

// A failed emission comes back as an Err instead of ending the stream, so
// matching on it lets the region emit a fallback in its place.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"Error handling"</h1>
        (live! {
            emit! { <p>"Loading..."</p> }?;
            match emit! { content() } {
                Err(error) => emit! {
                    <p>
                        "The content is unavailable: "
                        (error.to_string())
                    </p>
                },
                emitted => emitted,
            }
        })
    })
}

#[component]
async fn content() -> Result<impl View> {
    let content = load_content().await?;
    Ok(view! { <p>(content)</p> })
}

// Stands in for an upstream service that is down.
async fn load_content() -> Result<&'static str> {
    tokio::time::sleep(Duration::from_secs(1)).await;
    Err(std::io::Error::other("the upstream service is unreachable").into())
}
