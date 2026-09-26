[Datastar](https://data-star.dev) updates HTML and reactive signals from server responses. Use `data-*` attributes to bind signals to elements and actions such as `@get` to send requests.

Topcoat extracts signals from requests and creates Datastar responses. Return one update from a handler or send several over a [server-sent event stream](crate::router::content::sse).

Everything below is re-exported from `topcoat::datastar` and gated behind the `datastar` feature, which also enables the router's `sse` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.10.0", features = ["datastar"] }
```

# Loading the Datastar script

Load the Datastar script before using its attributes. This example bundles the script as a Topcoat asset:

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

See the [assets guide](crate::asset) for loading the asset bundle on your router.

# Reading signals

The [`Signals`] extractor deserializes request signals into your type. It reads JSON from the `datastar` query parameter for GET requests and from the body for other methods.

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

Use `Option<Signals<T>>` to also accept requests without Datastar. It returns `None` unless `Datastar-Request` is `true`. A Datastar request with malformed signals still returns an error. Use [`datastar_request`] to check the header directly.

# Patching elements

[`PatchElements`] updates page elements with HTML. By default, Datastar matches elements by `id` and morphs them to match the response. Use [`selector`](PatchElements::selector) and [`mode`](PatchElements::mode) to choose another target or update behavior. Returned from a handler, the patch sends one SSE event and closes the stream:

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

[`PatchElements::remove`] deletes elements matching a selector.

# Patching signals

[`PatchSignals`] merges a JSON object into the browser's signals. Set a value to `null` to remove it, or use [`only_if_missing`](PatchSignals::only_if_missing) to preserve existing values. [`PatchSignals::json`] serializes the object from a `Serialize` value.

# Streaming events

For live updates, return an [`Sse`] stream and convert each patch into an [`Event`] with `Into`. See the [server-sent events guide](crate::router::content::sse) for keep-alives and handling reconnections.

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

[`ExecuteScript`] appends a `<script>` element to the page. By default, the element removes itself after running.

```rust
use topcoat::datastar::ExecuteScript;

let script = ExecuteScript::new("console.log('saved')");
```

# Plain responses

Datastar also accepts ordinary responses. It patches `text/html` into the page and merges `application/json` into the signals. Response types that implement [`IntoResponseParts`] set headers to control these updates. Place them before the body in a response tuple:

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

# Header constants

The raw `datastar-*` header names are available as `HeaderName` constants in [`topcoat::datastar::header`](crate::datastar::header), for when you want to read or write a header directly.

[`Sse`]: crate::router::content::sse::Sse
[`Event`]: crate::router::content::sse::Event
[`IntoResponseParts`]: crate::router::response::IntoResponseParts
