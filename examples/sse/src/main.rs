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
    view::{View, view},
};

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .discover()
        .assets(AssetBundle::load().unwrap())
        .build();

    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
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

                // Connects to both endpoints and logs the events it receives.
                <script src=(asset!("./feed.js"))></script>
            </body>
        </html>
    })
}

#[route(GET "/ticks")]
async fn ticks(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    // A reconnecting browser sends the last ID it saw, so the stream resumes
    // where it left off.
    let next = last_event_id(cx)
        .and_then(|id| id.parse::<u64>().ok())
        .map_or(0, |last| last + 1);

    let events = stream::unfold(next, |tick| async move {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let event = Event::new()
            .event("tick")
            .id(tick.to_string())
            .retry(Duration::from_secs(1))
            .data(tick.to_string());

        Some((Ok(event), tick + 1))
    });

    // Keep-alive events hold the connection open while the stream is idle.
    Ok(Sse::new(events).keep_alive(KeepAlive::new()))
}

#[derive(Serialize)]
struct Progress {
    percent: u8,
}

#[route(GET "/job")]
async fn job() -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    // Unlike the ticks, this stream ends: the browser closes the connection
    // once it receives the `done` event.
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
