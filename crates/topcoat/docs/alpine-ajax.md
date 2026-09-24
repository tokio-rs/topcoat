[Alpine AJAX](https://alpine-ajax.js.org) updates parts of a page with HTML from the server. Add `x-target` to a form or link to select which elements the response replaces.

Topcoat reads Alpine AJAX's request headers so handlers can return the requested fragments. Configure browser behavior through Alpine AJAX's markup and events.

Everything below is re-exported from `topcoat::alpine_ajax` and gated behind the `alpine-ajax` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.9.0", features = ["alpine-ajax"] }
```

# Loading the Alpine AJAX script

Load the Alpine AJAX plugin before Alpine.js. Give both scripts `defer` so they initialize after the document is parsed:

```rust
use topcoat::{
    Result,
    router::{Slot, layout},
    view::{View, view},
};

#[layout]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <script defer="" src="https://cdn.jsdelivr.net/npm/@imacrayon/alpine-ajax@0.12.4/dist/cdn.min.js"></script>
                <script defer="" src="https://cdn.jsdelivr.net/npm/alpinejs@3.15.0/dist/cdn.min.js"></script>
            </head>
            <body>(slot)</body>
        </html>
    })
}
```

# Reading request headers

Use [`ajax_request`] to return a fragment for Alpine AJAX and a complete page for normal navigation:

```rust
use topcoat::{
    Result,
    alpine_ajax::ajax_request,
    context::Cx,
    router::{Slot, layout},
    view::{View, view},
};

#[layout]
async fn root(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        if ajax_request(cx) {
            // Return the elements Alpine AJAX will merge.
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

[`ajax_targets`] iterates over the requested target IDs. Use [`ajax_target`] to check whether one ID was requested.

# Sending alert messages with `x-sync`

Use `x-sync` for an alert region outside a form's targets. Alpine AJAX updates an element with `x-sync` whenever the response includes its ID, even if the element was not targeted or the response has an error status:

```html
<div id="alert" x-sync role="status"></div>

<form x-target="comment_form comments" method="post" action="/comments">
    <textarea name="body"></textarea>
    <button type="submit">"Post"</button>
</form>
```

Include `#alert` in the response from `/comments` to update the alert alongside the form's targets.

By default, `x-target` applies to successful responses. Add a status modifier to choose targets for errors. For example, `x-target.422` applies to a `422` response:

```html
<form
    id="comment_form"
    x-target="comment_form comments"
    x-target.422="comment_form"
    method="post"
    action="/comments"
>
    ...
</form>
```

Return `422` with the form and an error message when validation fails. The `.422` target list leaves `comments` unchanged. A successful response updates both targets:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{StatusCode, response::{IntoResponse, Response}, route},
    view::{ViewExt, view},
};

#[route(POST "/comments")]
async fn create_comment(cx: &Cx /* , Form(input): Form<NewComment> */) -> Result<Response> {
    let error: Option<&str> = None; // validate `input` here

    if let Some(message) = error {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            view! {
                cx =>
                <form id="comment_form" x-target="comment_form comments" x-target.422="comment_form">
                    <textarea name="body"></textarea>
                    <p class="error">(message)</p>
                    <button type="submit">"Post"</button>
                </form>
                <div id="alert" x-sync="" role="status">
                    <p>(message)</p>
                </div>
            }
            .single()
            .await?,
        )
            .into_response(cx);
    }

    // Save the comment and return the form, comments, and alert with status 200.
    # unreachable!()
}
```

# Header constants

The raw `X-Alpine-*` header names are available as `HeaderName` constants in [`topcoat::alpine_ajax::header`](crate::alpine_ajax::header), for when you want to read a header directly.
