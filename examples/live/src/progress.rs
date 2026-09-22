use std::time::Duration;

use topcoat::{
    Result,
    router::page,
    view::{View, emit, live, view},
};

// Every emit replaces the previous one, so the page can narrate a long-running
// task as it happens.
#[page]
pub async fn page() -> Result<impl View> {
    Ok(view! {
        <h1>"Progress"</h1>
        (live! {
            for percent in 0..100 {
                emit! {
                    <p>
                        "Working... "
                        (percent)
                        "%"
                    </p>
                }?;
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            emit! { <p>"Done!"</p> }
        })
    })
}
