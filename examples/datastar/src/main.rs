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
        href, page, route,
    },
    view::{View, ViewExt, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    // Datastar keeps the counter in the browser and sends it along with every
    // action request.
    Ok(view! {
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

                // The route's URL is interpolated into the Datastar action.
                <button data-on:click=(("@post('", href!(increment), "')"))>
                    "Increment"
                </button>

                <ol id="log"></ol>
            </body>
        </html>
    })
}

// Matches the signals declared by the page.
#[derive(Deserialize, Serialize)]
struct Counter {
    count: u64,
}

#[route(POST "/increment")]
async fn increment(
    cx: &Cx,
    Signals(counter): Signals<Counter>,
) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let count = counter.count + 1;

    let entry = view! {
        <li>
            "Counted to "
            (count)
        </li>
    }
    .single(cx)
    .await?;

    // One event updates the counter signal, the other appends the log entry.
    let events = stream::iter([
        PatchSignals::json(&Counter { count }).map(Into::into),
        Ok(PatchElements::new(entry.render(cx))
            .selector("#log")
            .mode(ElementPatchMode::Append)
            .into()),
    ]);

    Ok(Sse::new(events))
}
