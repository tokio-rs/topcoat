[htmx](https://htmx.org) is a small client-side library that lets HTML drive its own updates. Attributes like `hx-get` and `hx-post` make any element issue an HTTP request on an event (a click, a submit, an input) and swap the returned HTML fragment into the page, without a full reload and without writing JavaScript. The server just answers with the markup for the piece of the page that changed.

The browser and server coordinate this through a set of `HX-*` HTTP headers: the request carries headers describing what triggered it and where the response is headed, and the response can carry headers telling htmx how to apply the result. Topcoat helps you work with those request and response headers.

Everything below is re-exported from `topcoat::htmx` and gated behind the `htmx` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.1.2", features = ["htmx"] }
```

# Loading the htmx script

htmx is a client-side script the browser must load before any `hx-*` attribute does anything. You can point a `<script>` straight at a CDN, or vendor it as a Topcoat asset so it is self-hosted:

```rust
use topcoat::{
    Result,
    asset::asset,
    router::{Slot, layout},
    view::view,
};

#[layout]
async fn root(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <script src=(asset!("https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"))></script>
            </head>
            <body>(slot.await?)</body>
        </html>
    }
}
```

See the [assets guide](crate::asset) for loading the asset bundle on your router.

# Reading request headers

When htmx makes a request it sends a set of [request headers](https://htmx.org/reference/#request_headers) describing what triggered it and where the response should go. Each one has a matching accessor that reads it from the request context.

```rust
use topcoat::{
    Result,
    context::Cx,
    htmx::hx_request,
    router::{Slot, layout},
    view::view,
};

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result {
    // htmx only swaps out the target element, so we do not need to return
    // the full layout shell. Just the page's content are enough.
    if hx_request(cx) {
        return slot.await;
    }

    // Non-htmx requests require a full page render including the layout shell.
    view! {
        <html>
            <body>
                <nav> /* persistent navigation */ </nav>
                <main>(slot.await?)</main>
            </body>
        </html>
    }
}
```

The boolean headers have boolean accessors:

- [`hx_request`]: was this request issued by htmx?
- [`hx_boosted`]: did it come from an `hx-boost` element?
- [`hx_history_restore_request`]: is it restoring history after a cache miss?

The rest return the header as `Option<&str>`, borrowed straight from the request:

- [`hx_current_url`]: the browser's current URL.
- [`hx_prompt`]: the user's response to an `hx-prompt`.
- [`hx_target`]: the `id` of the target element.
- [`hx_trigger`]: the `id` of the triggering element.
- [`hx_trigger_name`]: the `name` of the triggering element.

# Setting response headers

htmx also reads a set of [response headers](https://htmx.org/reference/#response_headers) to redirect the browser, retarget the swap, refresh the page, or fire client-side events. Each one is a responder type implementing [`IntoResponseParts`], so you place it before the body in a handler's response tuple, exactly like a header array or a `StatusCode`.

```rust
use topcoat::{
    Result,
    context::Cx,
    htmx::{HxRetarget, HxReswap, SwapOption},
    router::route,
    view::{View, view},
};

#[route(POST "/save")]
async fn save(cx: &Cx) -> Result<(HxRetarget, HxReswap, View)> {
    let body = view! { <div>"Saved!"</div> }?;
    Ok((
        HxRetarget::from("#status"),
        HxReswap(SwapOption::InnerHtml),
        body,
    ))
}
```

The available responders:

- [`HxLocation`]: client-side redirect without a full reload, optionally with a fetch/swap context.
- [`HxPushUrl`] / [`HxReplaceUrl`]: update the browser history or location bar (or `prevent()` it).
- [`HxRedirect`]: client-side redirect to a new location.
- [`HxRefresh`]: force a full page refresh.
- [`HxReswap`]: override how the response is swapped in, via a [`SwapOption`].
- [`HxRetarget`] / [`HxReselect`]: retarget the swap, or reselect which part of the response is used.
- [`HxResponseTrigger`]: trigger client-side events, immediately or after the settle/swap step.

## Triggering client-side events

[`HxResponseTrigger`] fires named events on the client. With names alone it emits a comma-separated list; attach data to any event and it switches to the JSON form htmx expects.

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

[`IntoResponseParts`]: crate::router::IntoResponseParts
