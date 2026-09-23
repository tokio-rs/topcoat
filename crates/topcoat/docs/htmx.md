[htmx](https://htmx.org) updates parts of a page with HTML from the server. Add an attribute such as `hx-get` to send a request when the user interacts with an element.

Topcoat provides helpers to read htmx request headers and set response headers.

Enable the `htmx` feature to use `topcoat::htmx`:

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["htmx"] }
```

# Loading the htmx script

Load the htmx script in your layout. This example serves it as a Topcoat asset:

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
                <script src=(asset!("https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"))></script>
            </head>
            <body>(slot)</body>
        </html>
    })
}
```

See the [assets guide](crate::asset) for loading the asset bundle on your router.

# Reading request headers

Use [`hx_request`] to choose between a partial response for htmx and a full page:

```rust
use topcoat::{
    Result,
    context::Cx,
    htmx::hx_request,
    router::{Slot, layout},
    view::{View, view},
};

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        if hx_request(cx) {
            // Return the content to insert into the target element.
            (slot)
        } else {
            // Render the layout for a full page request.
            <html>
                <body>
                    <nav> /* persistent navigation */ </nav>
                    <main>(slot)</main>
                </body>
            </html>
        }
    })
}
```

Other request helpers read the corresponding htmx header. Boolean helpers return `false` when the header is absent. Text helpers return `Option<&str>` borrowed from the request.

# Setting response headers

Place an htmx response header before the body in a response tuple. For example, [`HxRetarget`] chooses the target element and [`HxReswap`] controls how to update it:

```rust
use topcoat::{
    Result,
    context::Cx,
    htmx::{HxRetarget, HxReswap, SwapOption},
    router::route,
    view::{ViewExt, ViewHandle, view},
};

#[route(POST "/save")]
async fn save(cx: &Cx) -> Result<(HxRetarget, HxReswap, ViewHandle)> {
    let body = view! { cx => <div>"Saved!"</div> }.single().await?;
    Ok((
        HxRetarget::from("#status"),
        HxReswap(SwapOption::InnerHtml),
        body,
    ))
}
```

Response header types implement [`IntoResponseParts`], so they compose with other response settings.

## Triggering client-side events

Use [`HxResponseTrigger`] to fire a browser event. Events can carry data and run at a chosen point in the update:

```rust
use topcoat::htmx::{HxEvent, HxResponseTrigger};

# fn example() -> topcoat::Result<HxResponseTrigger> {
// `HX-Trigger: item-saved`
let _ = HxResponseTrigger::receive(["item-saved"]);

// `HX-Trigger-After-Swap: {"show-toast":"Saved!"}`
let trigger = HxResponseTrigger::after_swap([
    HxEvent::with_data("show-toast", "Saved!")?,
]);
# Ok(trigger)
# }
```

# Header constants

Use the constants in [`header`](crate::htmx::header) to read or write raw headers.

[`IntoResponseParts`]: crate::router::response::IntoResponseParts
