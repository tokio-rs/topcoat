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
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
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

// The signals the page declares with its `data-signals:*` attributes. Datastar
// sends them along with every action request, and the same shape patches them.
#[derive(Deserialize, Serialize)]
struct Counter {
    count: u64,
}

// Bumps the counter and answers with two events: one patches the new count
// into the signals, the other appends an entry to the log.
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
    }?;

    let events = stream::iter([
        PatchSignals::json(&Counter { count }).map(Into::into),
        Ok(PatchElements::new(entry.render(cx))
            .selector("#log")
            .mode(ElementPatchMode::Append)
            .into()),
    ]);

    Ok(Sse::new(events))
}
