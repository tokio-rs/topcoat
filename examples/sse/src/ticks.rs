use futures_core::Stream;
use futures_util::stream;
use std::time::Duration;
use topcoat::{Result, context::Cx, router::{content::sse::{Event, KeepAlive, Sse, last_event_id}, route}};

#[route(GET)]
pub(crate) async fn ticks(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
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
