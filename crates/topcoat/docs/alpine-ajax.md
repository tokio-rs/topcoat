[Alpine AJAX](https://alpine-ajax.js.org) is an Alpine.js plugin that updates parts of a page with HTML from the server. Add `x-target` to a form or link to name the elements its response should update.

Topcoat provides helpers to identify Alpine AJAX requests and read their target element IDs.

Enable the `alpine-ajax` feature to use `topcoat::alpine_ajax`:

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["alpine-ajax"] }
```

# Loading the Alpine AJAX script

Load Alpine AJAX before Alpine.js. Use `defer` on both scripts so they run after the document has parsed:

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

Use [`ajax_request`] to return partial content for an Alpine AJAX request:

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
            // Return the content containing the requested targets.
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

Use [`ajax_targets`] to iterate over requested element IDs, or [`ajax_target`] to check for one ID.

# Sending alert messages with `x-sync`

Add `x-sync` to an alert region to update it whenever a response contains the same `id`. This works even when the element is outside the form's targets or the response has an error status:

```html
<div id="alert" x-sync role="status"></div>

<form x-target="comment_form comments" method="post" action="/comments">
    <textarea name="body"></textarea>
    <button type="submit">Post</button>
</form>
```

Include `#alert` in the response from `/comments` to update the message.

By default, `x-target` updates its targets only for successful responses. Add a status modifier to handle validation errors:

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

A `422` response updates only `comment_form`, while a successful response updates both targets. Include the form and its error message in the validation response:

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

    // Save the comment and return updated targets with a success status.
    # unreachable!()
}
```

# Header constants

Use the constants in [`header`](crate::alpine_ajax::header) to read raw headers.
