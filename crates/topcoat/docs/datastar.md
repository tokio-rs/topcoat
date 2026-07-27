[Datastar](https://data-star.dev) is a small client-side framework that drives page updates from the backend. `data-*` attributes bind reactive signals to elements, and actions like `@get` and `@post` send those signals to the server. The server answers with events that patch HTML elements and signal values into the page, either one at a time or as a long-lived stream.

Datastar consumes these events over [server-sent events](crate::router::content::sse), so this integration builds directly on the router's [`Sse`] response: the event types below convert into an SSE [`Event`], and each also works as a standalone response. On the request side, an extractor reads the signals every action sends along.

Everything below is re-exported from `topcoat::datastar` and gated behind the `datastar` feature, which also enables the router's `sse` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.4.0", features = ["datastar"] }
```

# Loading the Datastar script

Datastar is a client-side script the browser must load before any `data-*` attribute does anything. You can point a `<script>` straight at a CDN, or vendor it as a Topcoat asset so it is self-hosted:

```rust
use topcoat::{
    Result,
    asset::asset,
    router::layout,
    view::view,
};

#[layout]
async fn root(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <script
                    type="module"
                    src=(asset!("https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.2/bundles/datastar.js"))
                ></script>
            </head>
            <body>(slot?)</body>
        </html>
    }
}
```

See the [assets guide](crate::asset) for loading the asset bundle on your router.

# Reading signals

A Datastar action sends the page's signals with every request: GET requests carry them JSON-encoded in the `datastar` query parameter, all other requests as a JSON body. The [`Signals`] extractor reads them from either place and deserializes them into your type.

```rust
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    datastar::{PatchSignals, Signals},
    router::route,
};

#[derive(Deserialize, Serialize)]
struct Counter {
    count: u64,
}

#[route(POST "/increment")]
async fn increment(Signals(counter): Signals<Counter>) -> Result<PatchSignals> {
    PatchSignals::json(&Counter {
        count: counter.count + 1,
    })
}
```

Wrap the extractor in [`Option`] to also accept requests made without Datastar; it yields [`None`] when the request carries no `Datastar-Request` header. To branch on that header alone, [`datastar_request`] reads it from a `cx: &Cx`, just like its htmx counterpart.

# Patching elements

[`PatchElements`] carries HTML for Datastar to patch into the DOM. By default the elements are morphed into the page, matched by their `id` attribute; a [`selector`](PatchElements::selector) targets other elements and a [`mode`](PatchElements::mode) picks one of the [`ElementPatchMode`]s. Returned from a handler on its own, the patch responds as a stream that sends this one event and ends:

```rust
use topcoat::{
    Result,
    context::Cx,
    datastar::{ElementPatchMode, PatchElements},
    router::route,
    view::view,
};

#[route(POST "/entries")]
async fn create(cx: &Cx) -> Result<PatchElements> {
    let entry = view! { <li>"A new entry"</li> }?;
    Ok(PatchElements::new(entry.render(cx))
        .selector("#feed")
        .mode(ElementPatchMode::Prepend))
}
```

[`PatchElements::remove`] builds the inverse patch: it deletes the elements matching a selector from the page.

# Patching signals

[`PatchSignals`] updates the browser's signal store. The payload is a JSON object merged into the existing signals; a signal set to `null` is removed, and [`only_if_missing`](PatchSignals::only_if_missing) restricts the patch to signals that do not exist yet. [`PatchSignals::json`] serializes the payload from any `Serialize` value.

# Streaming events

For live updates, return an [`Sse`] stream and convert each patch into an [`Event`] with `Into`. Everything from the [server-sent events guide](crate::router::content::sse) applies: keep-alives fill idle gaps, and `Last-Event-ID` resumes a reconnected stream.

```rust
use futures_core::Stream;
use futures_util::stream;
use serde::Serialize;
use topcoat::{
    Result,
    datastar::PatchSignals,
    router::{
        content::sse::{Event, KeepAlive, Sse},
        route,
    },
};

#[derive(Serialize)]
struct Progress {
    percent: u8,
}

#[route(GET "/progress")]
async fn progress() -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
    let events = stream::iter((0..=100u8).step_by(20).map(|percent| {
        PatchSignals::json(&Progress { percent }).map(Into::into)
    }));

    Ok(Sse::new(events).keep_alive(KeepAlive::new()))
}
```

On the page, `data-on:load="@get('/progress')"` subscribes to the stream and applies each event as it arrives.

# Executing scripts

[`ExecuteScript`] runs JavaScript in the browser. It is sugar for a [`PatchElements`] that appends a `<script>` element to the `body`; by default the element removes itself after running.

```rust
use topcoat::datastar::ExecuteScript;

let script = ExecuteScript::new("console.log('saved')");
```

# Plain responses

Simple request-response updates do not need an event stream: Datastar also patches plain `text/html` responses into the DOM and merges plain `application/json` responses into the signals. A set of responder types implementing [`IntoResponseParts`] tunes how, by setting the response headers Datastar reads. Place one before the body in a handler's response tuple:

```rust
use topcoat::{
    Result,
    datastar::{DatastarMode, DatastarSelector, ElementPatchMode},
    router::route,
    view::{View, view},
};

#[route(POST "/save")]
async fn save() -> Result<(DatastarSelector, DatastarMode, View)> {
    let status = view! { <p>"Saved!"</p> }?;
    Ok((
        DatastarSelector::from("#status"),
        DatastarMode(ElementPatchMode::Inner),
        status,
    ))
}
```

The available responders:

- [`DatastarSelector`] / [`DatastarMode`] / [`DatastarUseViewTransition`]: target and shape how an HTML response is patched in.
- [`DatastarOnlyIfMissing`]: only patch signals from a JSON response that do not exist yet.
- [`DatastarScriptAttributes`]: set the script element attributes for a `text/javascript` response.

# Header constants

The raw `datastar-*` header names are available as `HeaderName` constants in [`topcoat::datastar::header`](crate::datastar::header), for when you want to read or write a header directly.

[`Sse`]: crate::router::content::sse::Sse
[`Event`]: crate::router::content::sse::Event
[`IntoResponseParts`]: crate::router::IntoResponseParts
