Server-sent events for Topcoat routes.

[Server-sent events](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events) (SSE) send a one-way stream of events from the server to the client over a normal HTTP response. This module needs the `sse` feature. It provides the [`Sse`] response, which wraps a `Stream` of [`Event`]s, responds with `Content-Type: text/event-stream`, and sends each event as soon as the stream yields it. In the browser, the built-in `EventSource` subscribes to the stream and reconnects by itself when the connection is lost.

# Streaming events

A route sends an event stream by returning an [`Sse`] that wraps the stream of events. Build an [`Event`] field by field. [`data`](Event::data) sets the payload, and [`json_data`](Event::json_data) serializes a value to JSON as the payload. [`event`](Event::event) sets the event type, which decides the listeners an `EventSource` passes the event to. [`id`](Event::id) and [`retry`](Event::retry) control reconnection.

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

The `use<>` bound makes sure the stream does not borrow the request context, which a route's response must not do. The connection stays open until the stream ends, the stream yields an `Err`, or the client disconnects. A disconnect drops the stream, so put cleanup code in the stream's `Drop`.

# Reading the request context

The stream lives on after the handler that returned it, so it cannot borrow the `Cx` the route was called with. Clone the `Cx` and move the clone into the stream instead. The clone reads the same app and request context.

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

Proxies and load balancers close connections that look idle. [`keep_alive`](Sse::keep_alive) sends events that the client ignores when the stream has been quiet for a while. [`KeepAlive::new`] sends an empty comment after 15 seconds without an event. Use [`interval`](KeepAlive::interval), [`text`](KeepAlive::text), and [`event`](KeepAlive::event) to change what is sent and when.

# Resuming after a reconnect

When an `EventSource` reconnects, it sends the [`id`](Event::id) of the last event it received in the `Last-Event-ID` request header. Read it with [`last_event_id`] to continue the stream where the client left off, instead of starting from the beginning.

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
