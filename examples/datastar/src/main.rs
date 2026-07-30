use futures_core::Stream;
use futures_util::stream;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
    datastar::{ElementPatchMode, PatchElements, PatchSignals, Signals},
    router::{
        Router, RouterBuilderDiscoverExt,
        content::sse::{Event, Sse},
        page, route,
    },
    view::view,
};

#[tokio::main]
async fn main() {
    // Discover the page and API route, then start the HTTP server.
    // By default, the application is available at http://127.0.0.1:3000.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
    // Datastar stores the counter in the browser and sends it to the server
    // whenever the increment action is triggered.
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Datastar"</title>

                <script
                    type="module"
                    src="https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.2/bundles/datastar.js"
                ></script>

                topcoat::dev::script()
            </head>

            <body data-signals:count="0">
                <h1>
                    "Count: "
                    <span data-text="$count"></span>
                </h1>

                <button data-on:click="@post('/increment')">"Increment"</button>

                <ol id="log"></ol>
            </body>
        </html>
    }
}

// This structure matches the signals declared by the page.
// Datastar sends the current values with every action request.
#[derive(Deserialize, Serialize)]
struct Counter {
    count: u64,
}

#[route(POST "/increment")]
async fn increment(
    cx: &Cx,
    Signals(counter): Signals<Counter>,
) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    // Calculate the next counter value using the signal received
    // from the browser.
    let count = counter.count + 1;

    // Create a new log entry that will be appended to the page.
    let entry = view! {
        <li>
            "Counted to "
            (count)
        </li>
    }?;

    // Send two Server-Sent Events:
    // one updates the counter signal and one appends the log entry.
    let events = stream::iter([
        PatchSignals::json(&Counter { count }).map(Into::into),
        Ok(PatchElements::new(entry.render(cx))
            .selector("#log")
            .mode(ElementPatchMode::Append)
            .into()),
    ]);

    Ok(Sse::new(events))
}
