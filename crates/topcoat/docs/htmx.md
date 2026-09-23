[htmx](https://htmx.org) is a small client-side library that lets HTML update itself. Attributes like `hx-get` and `hx-post` make an element send an HTTP request on an event (a click, a submit, an input) and swap the returned HTML fragment into the page. There is no full page reload and no JavaScript to write. The server answers with the markup for the part of the page that changed.

The browser and the server coordinate through `HX-*` HTTP headers. The request carries headers that describe what triggered it and where the response will go. The response can carry headers that tell htmx how to apply the result. This module gives you functions to read the request headers and types to set the response headers.

Everything below is re-exported from `topcoat::htmx` and gated behind the `htmx` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["htmx"] }
```

# Loading the htmx script

The browser must load the htmx script before any `hx-*` attribute does anything. You can point a `<script>` tag at a CDN, or declare the script as a Topcoat asset so that your app serves it itself:

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

See the [assets guide](crate::asset) for how to load the asset bundle on your router.

# Reading request headers

htmx sends [request headers](https://htmx.org/reference/#request_headers) that describe what triggered the request and where the response should go. Each header has a function that reads it from the request context:

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
            // htmx only swaps out the target element, so the layout shell is
            // not needed. The page content alone is enough.
            (slot)
        } else {
            // A normal browser request needs the full page, shell included.
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

These functions return `true` when the header is set to `true`, and `false` otherwise:

- [`hx_request`]: the request was sent by htmx.
- [`hx_boosted`]: the request came from an element using `hx-boost`.
- [`hx_history_restore_request`]: the request restores history after a miss in the local history cache.

These functions return the header value as an `Option<&str>`, borrowed from the request:

- [`hx_current_url`]: the current URL of the browser.
- [`hx_prompt`]: the user's response to an `hx-prompt`.
- [`hx_target`]: the `id` of the target element.
- [`hx_trigger`]: the `id` of the element that triggered the request.
- [`hx_trigger_name`]: the `name` of the element that triggered the request.

All of these functions panic when called outside a router request.

# Setting response headers

htmx reads [response headers](https://htmx.org/reference/#response_headers) that can redirect the browser, change where the response is swapped in, refresh the page, or trigger client-side events. Each header has a type that implements [`IntoResponseParts`]. Put it before the body in a handler's response tuple, the same way you would put a `StatusCode` or a header array there:

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

The available types:

- [`HxLocation`]: redirects on the client without a full page reload. It can also say how to fetch and swap the new content.
- [`HxPushUrl`] and [`HxReplaceUrl`]: push a URL onto the browser history, or replace the URL in the location bar. Their `prevent()` constructor stops htmx from changing it.
- [`HxRedirect`]: redirects on the client to a new location.
- [`HxRefresh`]: makes the browser do a full page refresh.
- [`HxReswap`]: changes how the response is swapped in, using a [`SwapOption`].
- [`HxRetarget`]: changes which element the response is swapped into.
- [`HxReselect`]: chooses which part of the response is swapped in.
- [`HxResponseTrigger`]: triggers client-side events, either right away or after the swap or settle step.

## Triggering client-side events

[`HxResponseTrigger`] triggers named events on the client. When no event carries data, the header is a comma-separated list of names. When at least one event carries data, the header becomes the JSON object that htmx expects, with `null` for events without data.

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

The raw `HX-*` header names are available as `HeaderName` constants in [`topcoat::htmx::header`](crate::htmx::header). Use them when you want to read or write a header yourself.

[`IntoResponseParts`]: crate::router::response::IntoResponseParts
