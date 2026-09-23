[Datastar](https://data-star.dev) updates pages from server responses. Its HTML attributes bind signals to elements and send requests. Topcoat can read those signals and return updates to page content or signal values.

Return a patch for one update, or convert patches into [`Event`] values for an [`Sse`] stream.

Enable the `datastar` feature to use `topcoat::datastar`. This also enables server-sent events:

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["datastar"] }
```

# Loading the Datastar script

Load the Datastar script in your layout. This example serves it as a Topcoat asset:

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

Use [`Signals`] to deserialize signals sent by a Datastar action. It reads the `datastar` query parameter for GET requests and the JSON body for other methods:

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

Use `Option<Signals<T>>` to accept requests without a `Datastar-Request` header. It returns `None` when that header is absent. Use [`datastar_request`] to check the header directly from `&Cx`.

# Patching elements

Return [`PatchElements`] to update page content. By default, it matches elements by `id` and updates them in place. Set a [`selector`](PatchElements::selector) and [`mode`](PatchElements::mode) to choose another target or operation:

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

Use [`PatchElements::remove`] to remove matching elements.

# Patching signals

Use [`PatchSignals::json`] to merge a serializable JSON object into the browser's signals. A `null` value removes a signal. Set [`only_if_missing`](PatchSignals::only_if_missing) to add signals without changing existing values.

# Streaming events

For live updates, return an [`Sse`] stream and convert each patch into an [`Event`]. See the [server-sent events guide](crate::router::content::sse) for stream configuration and reconnection handling.

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

Use [`ExecuteScript`] to run JavaScript in the browser. Its script element removes itself after execution by default.

```rust
use topcoat::datastar::ExecuteScript;

let script = ExecuteScript::new("console.log('saved')");
```

# Plain responses

Datastar also accepts plain HTML to update elements and JSON to update signals. Put response header types before the body in a tuple to control the update:

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

These header types implement [`IntoResponseParts`] and compose with other response settings.

# Header constants

Use the constants in [`header`](crate::datastar::header) to read or write raw headers.

[`Sse`]: crate::router::content::sse::Sse
[`Event`]: crate::router::content::sse::Event
[`IntoResponseParts`]: crate::router::response::IntoResponseParts
