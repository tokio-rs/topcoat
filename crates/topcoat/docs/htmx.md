[htmx](https://htmx.org) lets you drive page updates from HTML attributes, exchanging small fragments of markup over a handful of `HX-*` HTTP headers. Topcoat integrates with htmx the same way it handles the rest of a request: **accessor functions** read the request headers from a `cx: &Cx`, and **responders** that implement [`IntoResponseParts`] set the response headers.

Everything below is re-exported from `topcoat::htmx` and gated behind the `htmx` feature. There are no extractors, guards, or middleware — htmx is just more functions of `cx`.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.1", features = ["htmx"] }
```

# Reading request headers

When htmx makes a request it sends a set of [request headers](https://htmx.org/reference/#request_headers) describing what triggered it and where the response should go. Each one has a matching accessor that reads it from the request context.

```rust
use topcoat::{
    Result,
    context::Cx,
    htmx::{hx_request, hx_target},
    router::route,
    view::view,
};

#[route(GET "/items")]
async fn items(cx: &Cx) -> Result {
    // Serve just the list fragment to htmx, or the full page otherwise.
    if hx_request(cx) {
        return view! { <ul id="items"> /* … */ </ul> };
    }

    let _ = hx_target(cx); // the id of the element being updated, if any
    view! { <html> /* … full page … */ </html> }
}
```

The boolean headers have boolean accessors:

- [`hx_request`] — was this request issued by htmx?
- [`hx_boosted`] — did it come from an `hx-boost` element?
- [`hx_history_restore_request`] — is it restoring history after a cache miss?

The rest return the header as `Option<&str>`, borrowed straight from the request:

- [`hx_current_url`] — the browser's current URL.
- [`hx_prompt`] — the user's response to an `hx-prompt`.
- [`hx_target`] — the `id` of the target element.
- [`hx_trigger`] — the `id` of the triggering element.
- [`hx_trigger_name`] — the `name` of the triggering element.

These reads are plain header lookups, so they are not memoized: borrowing the value is cheaper than the owned copy a cache would hold. Reach for [`memoize`](crate::context::memoize) on the *function that uses* the header (a user lookup, a feature-flag check) rather than on the header read itself.

# Setting response headers

htmx also reads a set of [response headers](https://htmx.org/reference/#response_headers) to redirect the browser, retarget the swap, refresh the page, or fire client-side events. Each one is a responder type implementing [`IntoResponseParts`], so you place it before the body in a handler's response tuple — exactly like a header array or [`StatusCode`].

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

- [`HxLocation`] — client-side redirect without a full reload, optionally with a fetch/swap context.
- [`HxPushUrl`] / [`HxReplaceUrl`] — update the browser history or location bar (or `prevent()` it).
- [`HxRedirect`] — client-side redirect to a new location.
- [`HxRefresh`] — force a full page refresh.
- [`HxReswap`] — override how the response is swapped in, via a [`SwapOption`].
- [`HxRetarget`] / [`HxReselect`] — retarget the swap, or reselect which part of the response is used.
- [`HxResponseTrigger`] — trigger client-side events, immediately or after the settle/swap step.

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
