Server-sent events for Topcoat routes.

Server-sent events (SSE) send events from the server to a client over one HTTP response. Enable the `sse` feature and return an [`Sse`] response containing a stream of [`Event`]s. In a browser, use `EventSource` to subscribe and reconnect when the connection is lost.

# Streaming events

Return [`Sse`] with the stream of events to send. Build each [`Event`] with [`data`](Event::data) for text or [`json_data`](Event::json_data) for a serialized value. Optional fields can name the event and control reconnection.

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

The `use<>` bound prevents the returned stream from borrowing the request context. The response ends when the stream finishes, yields an error, or the client disconnects. Put cleanup in the stream's `Drop` implementation so it also runs on disconnect.

# Reading the request context

A stream outlives the handler that returned it, so it cannot borrow the `Cx` the route was called with. Clone the `Cx` and move the owned handle into the stream instead; it reads the same app and request context.

```rust
use futures_core::Stream;
use topcoat::{
    Result,
    context::{Cx, request_context},
    router::{
        content::sse::{Event, Sse},
        route,
    },
};

struct Customer {
    name: String,
}

#[route(GET "/greetings")]
async fn greetings(cx: &Cx) -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let cx = cx.clone();
    let events = futures_util::stream::once(async move {
        let customer: &Customer = request_context(&cx);
        Ok(Event::new().data(customer.name.as_str()))
    });
    Ok(Sse::new(events))
}
```

# Keeping quiet streams alive

Proxies and load balancers may close idle connections. Configure [`keep_alive`](Sse::keep_alive) to send comments while no events are ready. Use [`KeepAlive`] to choose the interval and content.

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
