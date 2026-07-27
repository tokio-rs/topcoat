Server-sent events for topcoat routes.

Server-sent events (SSE) push a one-way stream of events from the server to the client over a plain HTTP response. This module (behind the `sse` feature) provides the [`Sse`] response: it wraps a `Stream` of [`Event`]s, replies with `Content-Type: text/event-stream`, and sends each event as the stream yields it. On the client, the browser's built-in `EventSource` subscribes to the stream and reconnects on its own when the connection is lost.

# Streaming events

A route becomes an event stream by returning [`Sse`] wrapping the stream of events to send. An [`Event`] is assembled field by field: [`data`](Event::data) carries the payload ([`json_data`](Event::json_data) serializes a value to JSON), [`event`](Event::event) names the type an `EventSource` dispatches to its listeners, and [`id`](Event::id) and [`retry`](Event::retry) drive reconnection.

```rust
use futures_core::Stream;
use topcoat::{
    Result,
    router::{
        content::sse::{Event, KeepAlive, Sse},
        route,
    },
};

#[route(GET "/events")]
async fn events() -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let events = futures_util::stream::iter(
        ["one", "two", "three"].map(|name| Ok(Event::new().event("named").data(name))),
    );
    Ok(Sse::new(events).keep_alive(KeepAlive::new()))
}
```

The `use<>` bound keeps the stream from capturing the request context, which a route's response must not borrow. The connection stays open until the stream ends, an `Err` item occurs, or the client disconnects; a disconnect drops the stream, so tie cleanup to the stream's `Drop`.

# Keeping quiet streams alive

Proxies and load balancers drop connections that look stale. [`keep_alive`](Sse::keep_alive) fills idle gaps with events the client ignores: [`KeepAlive::new`] sends an empty comment after 15 idle seconds, and [`interval`](KeepAlive::interval), [`text`](KeepAlive::text), and [`event`](KeepAlive::event) tune what is sent and when.

# Resuming after a reconnect

A reconnecting `EventSource` echoes the [`id`](Event::id) of the last event it received in the `Last-Event-ID` request header. Read it with [`last_event_id`] to resume the stream where the client left off instead of replaying it from the start.

```rust
use futures_core::Stream;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::sse::{Event, Sse, last_event_id},
        route,
    },
};

#[route(GET "/ticks")]
async fn ticks(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let next: u64 = last_event_id(cx)
        .and_then(|id| id.parse().ok())
        .map_or(0, |last: u64| last + 1);
    let events = futures_util::stream::iter(
        (next..next + 3).map(|tick| Ok(Event::new().id(tick.to_string()).data(tick.to_string()))),
    );
    Ok(Sse::new(events))
}
```
