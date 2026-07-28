use std::time::Duration;

use futures_core::Stream;
use futures_util::stream;
use serde::Serialize;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt, asset},
    context::Cx,
    router::{
        Router, RouterBuilderDiscoverExt,
        content::sse::{Event, KeepAlive, Sse, last_event_id},
        page, route,
    },
    view::view,
};

#[tokio::main]
async fn main() {
    // Discover the routes, load the generated browser assets, and build the router.
    let router = Router::builder()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build();

    // The application is available at http://127.0.0.1:3000 by default.
    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Server-sent events"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"Server-sent events"</h1>

                // Start a finite stream that reports the progress of a job.
                <button id="start">"Run a job"</button>

                // Events received by the browser are appended to this list.
                <ul id="log"></ul>

                // Load the browser code that connects to the SSE endpoints.
                <script src=(asset!("./feed.js"))></script>
            </body>
        </html>
    }
}

#[route(GET "/ticks")]
async fn ticks(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    // Resume after the last event received by the browser, or begin at zero.
    let next = last_event_id(cx)
        .and_then(|id| id.parse::<u64>().ok())
        .map_or(0, |last| last + 1);

    // Produce one named `tick` event every second.
    let events = stream::unfold(next, |tick| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let event = Event::new()
            .event("tick")
            .id(tick.to_string())
            .retry(Duration::from_secs(1))
            .data(tick.to_string());

        Some((Ok(event), tick + 1))
    });

    // Keep the connection alive while the stream is open.
    Ok(Sse::new(events).keep_alive(KeepAlive::new()))
}

#[derive(Serialize)]
struct Progress {
    percent: u8,
}

#[route(GET "/job")]
async fn job() -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    // Send progress updates from zero to 90 percent, followed by a done event.
    let events = stream::unfold(0, |percent| async move {
        if percent > 100 {
            return None;
        }

        tokio::time::sleep(Duration::from_millis(400)).await;

        let event = if percent == 100 {
            Ok(Event::new().event("done").data("finished"))
        } else {
            Event::new()
                .event("progress")
                .json_data(&Progress { percent })
        };

        Some((event, percent + 10))
    });

    Ok(Sse::new(events))
}
