[htmx](https://htmx.org) updates parts of a page with HTML from the server. Attributes such as `hx-get` send requests and select where the response appears.

Topcoat reads htmx request headers and sets response headers that control how the browser applies an update.

Everything below is re-exported from `topcoat::htmx` and gated behind the `htmx` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.10.0", features = ["htmx"] }
```

# Loading the htmx script

Load the htmx script before using `hx-*` attributes. This example bundles the script as a Topcoat asset:

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

Use [`hx_request`] to distinguish htmx requests from full-page requests:

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
            // Return the fragment htmx will insert.
            (slot)
        } else {
            // Return a complete page for normal navigation.
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

Other accessors read individual [request headers](https://htmx.org/reference/#request_headers). For example, [`hx_target`] returns the target element's ID as an `Option<&str>`.

# Setting response headers

Use response types to set htmx [response headers](https://htmx.org/reference/#response_headers). They implement [`IntoResponseParts`], so place them before the body in the response tuple. This example changes the target and swap mode:

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

## Triggering client-side events

[`HxResponseTrigger`] fires named events in the browser. Events can include data and run at different stages of the update:

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

The raw `HX-*` header names are available as `HeaderName` constants in [`topcoat::htmx::header`](crate::htmx::header), for when you want to read or write a header directly.

[`IntoResponseParts`]: crate::router::response::IntoResponseParts
