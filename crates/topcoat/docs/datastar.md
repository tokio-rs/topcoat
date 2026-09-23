[Datastar](https://data-star.dev) is a small client-side framework where the server drives page updates. `data-*` attributes bind reactive signals to elements, and actions like `@get` and `@post` send those signals to the server. The server answers with events that patch HTML elements and signal values into the page. It can send one event or keep a stream open and send many.

Datastar receives these events as [server-sent events](crate::router::content::sse), so this module builds on the router's [`Sse`] response. Each event type below converts into an SSE [`Event`], and each can also be returned from a handler on its own. On the request side, an extractor reads the signals that every action sends.

Everything below is re-exported from `topcoat::datastar` and gated behind the `datastar` feature, which also turns on the router's `sse` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["datastar"] }
```

# Loading the Datastar script

The browser must load the Datastar script before any `data-*` attribute does anything. You can point a `<script>` tag at a CDN, or declare the script as a Topcoat asset so that your app serves it itself:

```rust
use topcoat::{
    Result,
    asset::asset,
    router::{Slot, layout},
    view::{View, view},
};

#[layout]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <script
                    type="module"
                    src=(asset!("https://cdn.jsdelivr.net/gh/starfederation/datastar@1.0.2/bundles/datastar.js"))
                ></script>
            </head>
            <body>(slot)</body>
        </html>
    })
}
```

See the [assets guide](crate::asset) for how to load the asset bundle on your router.

# Reading signals

A Datastar action sends the page's signals with every request, except signals whose name starts with an underscore. GET requests carry them as JSON in the `datastar` query parameter. All other requests carry them as a JSON body. The [`Signals`] extractor reads them from either place and deserializes them into your type:

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

To also accept requests that do not come from Datastar, wrap the extractor in [`Option`]. It yields [`None`] when the request has no `Datastar-Request` header, and still returns an error when the signals cannot be read. To check for that header without reading the signals, call [`datastar_request`] with a `cx: &Cx`.

# Patching elements

[`PatchElements`] sends HTML for Datastar to patch into the page. By default, Datastar morphs each element into the existing element with the same `id`. Call [`selector`](PatchElements::selector) to target other elements, and [`mode`](PatchElements::mode) to pick a different [`ElementPatchMode`]. When a handler returns a patch on its own, the response is a stream that sends this one event and then ends:

```rust
use topcoat::{
    Result,
    context::Cx,
    datastar::{ElementPatchMode, PatchElements},
    router::route,
    view::{ViewExt, view},
};

#[route(POST "/entries")]
async fn create(cx: &Cx) -> Result<PatchElements> {
    let entry = view! { cx => <li>"A new entry"</li> }.single().await?;
    Ok(PatchElements::new(entry.render(cx))
        .selector("#feed")
        .mode(ElementPatchMode::Prepend))
}
```

[`PatchElements::remove`] creates a patch that removes the elements matching a selector from the page.

# Patching signals

[`PatchSignals`] updates the signals in the browser. The payload is a JSON object that is merged into the existing signals. A signal set to `null` is removed. [`PatchSignals::json`] serializes the payload from any `Serialize` value, and [`only_if_missing`](PatchSignals::only_if_missing) limits the patch to signals that do not exist yet.

```rust
use serde_json::json;
use topcoat::datastar::PatchSignals;

# fn example() -> topcoat::Result<PatchSignals> {
// Sets `count`, removes `draft`, and sets `theme` only if it is not set yet.
let patch = PatchSignals::json(&json!({ "count": 0, "draft": null }))?;
let defaults = PatchSignals::json(&json!({ "theme": "dark" }))?.only_if_missing(true);
# let _ = patch;
# Ok(defaults)
# }
```

# Streaming events

For live updates, return an [`Sse`] stream and convert each patch into an [`Event`] with `Into`. Everything in the [server-sent events guide](crate::router::content::sse) applies here, such as keep-alive events for idle streams and resuming with `Last-Event-ID`. Each event type has `id` and `retry` methods for this.

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

On the page, `data-init="@get('/progress')"` opens the stream when the element loads, and Datastar applies each event as it arrives.

# Executing scripts

[`ExecuteScript`] runs JavaScript in the browser. It sends a [`PatchElements`] event that appends a `<script>` element to the `body`. By default the element removes itself after it runs.

```rust
use topcoat::datastar::ExecuteScript;

let script = ExecuteScript::new("console.log('saved')");
```

# Plain responses

Simple updates do not need an event stream. Datastar also patches a plain `text/html` response into the page, merges a plain `application/json` response into the signals, and runs a plain `text/javascript` response as a script. A few types implementing [`IntoResponseParts`] set the response headers that control how Datastar applies these responses. Put one before the body in a handler's response tuple:

```rust
use topcoat::{
    Result,
    context::Cx,
    datastar::{DatastarMode, DatastarSelector, ElementPatchMode},
    router::route,
    view::{ViewExt, ViewHandle, view},
};

#[route(POST "/save")]
async fn save(cx: &Cx) -> Result<(DatastarSelector, DatastarMode, ViewHandle)> {
    let status = view! { cx => <p>"Saved!"</p> }.single().await?;
    Ok((
        DatastarSelector::from("#status"),
        DatastarMode(ElementPatchMode::Inner),
        status,
    ))
}
```

The available types:

- [`DatastarSelector`]: the elements that an HTML response patches, as a CSS selector.
- [`DatastarMode`]: how an HTML response is patched in.
- [`DatastarUseViewTransition`]: whether an HTML response is patched in using the View Transition API.
- [`DatastarOnlyIfMissing`]: whether a JSON response only patches signals that do not exist yet.
- [`DatastarScriptAttributes`]: the attributes of the script element for a JavaScript response.

# Header constants

The raw `datastar-*` header names are available as `HeaderName` constants in [`topcoat::datastar::header`](crate::datastar::header). Use them when you want to read or write a header yourself.

[`Sse`]: crate::router::content::sse::Sse
[`Event`]: crate::router::content::sse::Event
[`IntoResponseParts`]: crate::router::response::IntoResponseParts
