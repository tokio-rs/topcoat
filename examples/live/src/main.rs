use std::time::Duration;

use topcoat::{
    Result,
    router::{Router, RouterBuilderDiscoverExt, Slot, href, layout, page},
    view::{View, emit, live, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[layout("/")]
async fn shell(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Live"</title>

                // Reloads the browser when the dev server rebuilds the app.
                topcoat::dev::script()
            </head>
            <body>
                <nav>
                    <a href=(href!(quote))>"Quote"</a>
                    " | "
                    <a href=(href!(progress))>"Progress"</a>
                </nav>
                (slot)
            </body>
        </html>
    })
}

// The browser receives the shell with the loading message right away, and the
// quote replaces it in place once the lookup finishes.
#[page("/")]
async fn quote() -> Result<impl View> {
    Ok(view! {
        <h1>"Quote of the day"</h1>
        (live! {
            emit! { <p>"Loading..."</p> }?;
            let quote = fetch_quote().await;
            emit! { <blockquote>(quote)</blockquote> }
        })
    })
}

// Stands in for a slow database query or upstream request.
async fn fetch_quote() -> &'static str {
    tokio::time::sleep(Duration::from_secs(2)).await;
    "Simplicity is prerequisite for reliability."
}

// Every emit replaces the previous one, so the page can narrate a long-running
// task as it happens.
#[page("/progress")]
async fn progress() -> Result<impl View> {
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
