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
    let router = Router::builder()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build();

    topcoat::start(router).await.unwrap();
}

// The page subscribes to both streams with the browser's EventSource.
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
                <button id="start">"Run a job"</button>
                <ul id="log"></ul>
                <script src=(asset!("./feed.js"))></script>
            </body>
        </html>
    }
}

// --- An endless stream ------------------------------------------------------

// Sends a tick every second, forever. The `use<>` bound keeps the stream from
// capturing the request context, which a response must not borrow.
#[route(GET "/ticks")]
async fn ticks(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    // A reconnecting EventSource echoes the id of the last event it received,
    // so the stream continues counting instead of starting over.
    let next = last_event_id(cx)
        .and_then(|id| id.parse::<u64>().ok())
        .map_or(0, |last| last + 1);

    let events = stream::unfold(next, |tick| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        // The event name picks the client listener, the id is what the client
        // echoes back on a reconnect, and the retry sets how long it waits
        // before reconnecting.
        let event = Event::new()
            .event("tick")
            .id(tick.to_string())
            .retry(Duration::from_secs(1))
            .data(tick.to_string());

        Some((Ok(event), tick + 1))
    });

    // Without traffic, a proxy is free to drop the connection as stale. The
    // keep-alive fills idle gaps with comments the client ignores.
    Ok(Sse::new(events).keep_alive(KeepAlive::new()))
}

// --- A stream that ends -----------------------------------------------------

#[derive(Serialize)]
struct Progress {
    percent: u8,
}

// Reports the progress of a job and then ends, which closes the connection.
#[route(GET "/job")]
async fn job() -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let events = stream::unfold(0, |percent| async move {
        if percent > 100 {
            return None;
        }

        tokio::time::sleep(Duration::from_millis(400)).await;

        // json_data serializes the payload and reports a failure as a stream
        // error, which ends the response body.
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
