This module provides Topcoat's HTML templating primitives:

- [`view!`]: the HTML-like templating macro.
- [`#[component]`][`component`]: turns an async function into a reusable component with typed props and child content.
- [`attributes!`]: builds a reusable runtime [`Attributes`] value from the same attribute syntax used inside [`view!`].
- [`class!`]: space-separated class lists from static and conditional entries.

# Deferred views

Any [`View`] can carry work that finishes after its response shell. Add [`defer_script`] to the document head, then use `defer component() { placeholder }` inside [`view!`]:

```rust
use topcoat::{
    Result,
    view::{component, defer_script, view},
};

#[component]
async fn activity() -> Result {
    view! { <p>"Activity is ready."</p> }
}

#[component]
async fn dashboard() -> Result {
    view! {
        <html>
            <head>defer_script()</head>
            <body>
                defer activity() {
                    <p aria-busy="true">"Loading activity..."</p>
                }
            </body>
        </html>
    }
}
```

The placeholder is part of the first response chunk. Deferred components run concurrently after the response body starts, and each completed view is streamed in completion order. The browser helper is a bundled external script. It runs while the document is parsing so each streamed fragment is applied as it arrives. Streamed chunks contain inert, annotated `<template>` elements rather than inline scripts.

The asset bundle must be loaded on the router so [`defer_script`] has a URL. See the [`asset`](crate::asset) guide for setup.

Use [`View::defer`] when the work is not a direct component call:

```rust
# use topcoat::{Result, view::{component, view}};
# async fn load_activity() -> Result { view! { <p>"ready"</p> } }
# #[component]
# async fn example() -> Result {
let activity = view! {
    <p aria-busy="true">"Loading activity..."</p>
}?
.defer(|cx| async move {
    let cx = cx.as_ref();
    view! { cx => (load_activity().await?) }
});

view! { <section>(activity)</section> }
# }
```

Deferred views compose as ordinary views. A component can return a view containing deferred work, and a parent can insert that component normally. The final response discovers the nested work automatically; there is no separate streaming view type or include helper.

The response body owns each deferred future. Dropping the body drops pending work. Status codes and headers come from the initial shell because deferred views complete after response headers may be sent. Calling [`View::render`] directly renders the placeholder markers but does not run deferred work; return the view through the router to stream it.

# Streaming JSON

Components can send serialized data to browser modules separately from their HTML. This lets a module begin loading immediately and await its data by key:

```rust
# use topcoat::{Result, context::Cx, view::{component, view}};
# #[component]
# async fn product_search(cx: &Cx) -> Result {
let products = ["Anvil", "Drill", "Level"];
let key = cx.send_json(&products)?;

view! {
    <input data-products=(key.as_str())>
}
# }
```

Browser code reads the attribute and awaits the streamed value:

```js
const input = document.querySelector("[data-products]");
const products = await topcoat.json(input.dataset.products);
```

[`Cx::send_json`](crate::context::Cx::send_json) generates a response-scoped key in a namespace applications cannot use, so generated and named keys cannot collide. Use [`Cx::send_json_named`](crate::context::Cx::send_json_named) when client code needs a stable key. Repeating a named key with the same serialized value is deduplicated; sending a different value under that key is an error.

JSON is serialized on the server and streamed in an inert, HTML-escaped template. The external [`defer_script`] helper parses it and resolves `topcoat.json(key)`, including calls made before the data arrives.

[`view!`]: macro.view.html
[`component`]: attr.component.html
[`attributes!`]: macro.attributes.html
[`Attributes`]: struct.Attributes.html
[`class!`]: macro.class.html
[`defer_script`]: fn.defer_script.html
[`View`]: struct.View.html
[`View::defer`]: struct.View.html#method.defer
