[Alpine AJAX](https://alpine-ajax.js.org) is an [Alpine.js](https://alpinejs.dev) plugin that lets HTML update itself. Attributes like `x-target` make a `<form>` or `<a>` send a fetch request and merge the returned HTML into one or more target elements. There is no full page reload and no JavaScript to write. The server answers with the markup for the part of the page that changed.

Alpine AJAX tells the server about a request through two `X-Alpine-*` HTTP headers. Unlike htmx, it has no response headers. Merging, navigation, and client-side events are all set up in markup (`x-target`, `x-merge`) or in JavaScript (`ajax:*` events). This module gives you functions to read the request headers.

Everything below is re-exported from `topcoat::alpine_ajax` and gated behind the `alpine-ajax` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.8.1", features = ["alpine-ajax"] }
```

# Loading the Alpine AJAX script

Alpine AJAX is a plugin for Alpine.js, so the browser must load both scripts in this order: first the plugin, then Alpine.js itself. Load both with `defer` so that they run after the document has been parsed. Without `defer`, Alpine can start before `<body>` exists and skip the directives on the first render without any error.

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

Alpine AJAX sends two [request headers](https://alpine-ajax.js.org/reference/): `X-Alpine-Request` marks the request as coming from Alpine AJAX, and `X-Alpine-Target` lists the `id`s of the target elements. Each has a function that reads it from the request context:

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
            // Alpine AJAX only merges the target elements, so the layout
            // shell is not needed. The page content alone is enough.
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

- [`ajax_request`]: returns `true` when the request was sent by Alpine AJAX.
- [`ajax_targets`]: returns an iterator over the `id`s of the target elements.
- [`ajax_target`]: returns `true` when a given `id` is one of the target elements.

All of these functions panic when called outside a router request.

# Sending alert messages with `x-sync`

A form's `x-target` only merges the elements it names. The `x-sync` attribute covers everything else: an element with `x-sync` is updated whenever a response contains an element with the same `id`, even when that element is not a target. This makes it a good fit for a flash message or alert area that lives outside of what a form targets:

```html
<div id="alert" x-sync role="status"></div>

<form x-target="comment_form comments" method="post" action="/comments">
    <textarea name="body"></textarea>
    <button type="submit">Post</button>
</form>
```

Include the `#alert` markup in every response from `/comments`, on success and on failure. Alpine AJAX replaces the alert in place, even though it is not in the form's `x-target`.

Together with the status code modifiers of `x-target`, this covers form validation. By default, `x-target` merges its targets for any response, error responses included. A modifier sets a different target list for some status codes: `x-target.422` applies to a `422` response, `x-target.4xx` to any `4xx` response, and `x-target.error` to any `4xx` or `5xx` response.

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

When validation fails, the handler responds with a `422` and only the `#comment_form` fragment, which holds the textarea and an inline error. The `comments` list stays as it is because it is not in the `.422` target list. When the comment is saved, the handler responds with a `200` and both targets are updated.

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

    // Save the comment, then respond with the cleared form, the updated
    // `comments` list, and an `#alert` confirmation, like above but with `200`.
    # unreachable!()
}
```

# Header constants

The raw `X-Alpine-*` header names are available as `HeaderName` constants in [`topcoat::alpine_ajax::header`](crate::alpine_ajax::header). Use them when you want to read a header yourself.
