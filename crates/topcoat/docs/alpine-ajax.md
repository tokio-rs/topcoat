[Alpine AJAX](https://alpine-ajax.js.org) is an Alpine.js plugin that lets HTML drive its own updates. Attributes like `x-target` make a `<form>` or `<a>` issue a fetch request and merge the returned HTML fragment into one or more target elements, without a full reload and without writing JavaScript. The server just answers with the markup for the piece of the page that changed.

The browser tells the server about the request through two `X-Alpine-*` HTTP headers. Unlike htmx, Alpine AJAX defines no response header convention: there is nothing for the server to set to control navigation, retargeting, or client-side events -- those are all configured directly in markup (`x-merge`, `x-target`) or JavaScript (`ajax:*` events). Topcoat helps you read the request headers.

Everything below is re-exported from `topcoat::alpine_ajax` and gated behind the `alpine-ajax` feature.

```toml
# Cargo.toml
[dependencies]
topcoat = { version = "0.5.0", features = ["alpine-ajax"] }
```

# Loading the Alpine AJAX script

Alpine AJAX is a plugin for Alpine.js core, so the browser must load both, in order: the plugin script first, then Alpine.js itself. Load both with `defer` so they run after the document has parsed; without it, Alpine can start initializing before `<body>` exists and silently skip binding directives on the page's first render.

```rust
use topcoat::{
    Result,
    router::layout,
    view::view,
};

#[layout]
async fn root(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <script defer="" src="https://cdn.jsdelivr.net/npm/@imacrayon/alpine-ajax@0.12.4/dist/cdn.min.js"></script>
                <script defer="" src="https://cdn.jsdelivr.net/npm/alpinejs@3.15.0/dist/cdn.min.js"></script>
            </head>
            <body>(slot?)</body>
        </html>
    }
}
```

# Reading request headers

When Alpine AJAX makes a request it sends two headers describing itself and what it's targeting.

```rust
use topcoat::{
    Result,
    alpine_ajax::ajax_request,
    context::Cx,
    router::layout,
    view::view,
};

#[layout]
async fn root(cx: &Cx, slot: Result) -> Result {
    // Alpine AJAX only merges the requested target elements, so we do not
    // need to return the full layout shell. Just the page's content is
    // enough.
    if ajax_request(cx) {
        return slot;
    }

    // Non-AJAX requests require a full page render including the layout shell.
    view! {
        <html>
            <body>
                <nav> /* persistent navigation */ </nav>
                <main>(slot?)</main>
            </body>
        </html>
    }
}
```

- [`ajax_request`]: was this request issued by Alpine AJAX?
- [`ajax_targets`]: an iterator over the `id`s of the target elements being requested.
- [`ajax_target`]: is a given `id` among the requested targets?

# Sending alert messages with `x-sync`

A form's `x-target` only merges the elements it names, and by default only for a `2xx` response. Alpine AJAX's `x-sync` attribute is the escape hatch: any element on the page carrying `x-sync` is refreshed whenever a response contains a matching `id`, regardless of whether that `id` was targeted and regardless of status code. That makes it the right tool for a flash message or alert region that lives outside whatever a form explicitly targets:

```html
<div id="alert" x-sync role="status"></div>

<form x-target="comment_form comments" method="post" action="/comments">
    <textarea name="body"></textarea>
    <button type="submit">"Post"</button>
</form>
```

Render the `#alert` markup into every response from `/comments` (success or failure) and Alpine AJAX picks it up and replaces it in place, without it ever appearing in that form's `x-target`.

Pairing this with `x-target`'s status-code modifiers covers form validation end to end. Add a modifier so the target list changes on a non-2xx response -- `x-target.422` merges only on a `422`, `x-target.4xx` on any `4xx`, `x-target.error` on `4xx` or `5xx`:

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

A validation failure can respond `422` with just the `#comment_form` fragment (the textarea and an inline error) -- `comments` is left alone because it isn't in the `.422` target list -- while a success responds `200` and updates both. The Rust attribute name needs the parenthesized-expression form, since `422` on its own is not a valid identifier segment:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{IntoResponse, Response, StatusCode, route},
    view::view,
};

#[route(POST "/comments")]
async fn create_comment(cx: &Cx /* , Form(input): Form<NewComment> */) -> Result<Response> {
    let error: Option<&str> = None; // validate `input` here

    if let Some(message) = error {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            view! {
                <form id="comment_form" x-target="comment_form comments" x-target.422="comment_form">
                    <textarea name="body"></textarea>
                    <p class="error">(message)</p>
                    <button type="submit">"Post"</button>
                </form>
                <div id="alert" x-sync="" role="status">
                    <p>(message)</p>
                </div>
            }?,
        )
            .into_response(cx);
    }

    // ...save the comment, then respond with the cleared form, the updated
    // `comments` list, and an `#alert` confirmation, same as above but `200`.
    # unreachable!()
}
```

# Header constants

The raw `X-Alpine-*` header names are available as `HeaderName` constants in [`topcoat::alpine_ajax::header`](crate::alpine_ajax::header), for when you want to read a header directly.
