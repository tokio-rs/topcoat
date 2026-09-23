Declares a page: a handler that renders a view.

The attribute takes an optional path string. With an absolute path, such as `#[page("/about")]`, the page is served at that path. Without a path, the page is served at the path of its module, as described in [`module_router!`](macro.module_router.html). A path that starts with `./` is added to the end of the module path, so `#[page("./export")]` in `src/app/settings.rs` serves `/settings/export`. One application can mix pages with absolute and module-derived paths.

A page serves `GET` by default. To serve other methods, name them before the path, in the same forms as [`#[route]`](attr.route.html): one method (`#[page(POST "/signup")]`), a bracketed list (`[GET, POST]`), or `*` for every method.

A path string is a Topcoat [`Path`](struct.Path.html). It can hold literal segments (`users`), `{name}` for a dynamic parameter, `{*name}` for a catch-all that matches the rest of the path, and `(name)` for a group. A group takes part in layout and layer matching but is not part of the served URL.

Register a page with an absolute path by passing its name to [`RouterBuilder::page`](struct.RouterBuilder.html#method.page), or let [`discover`](trait.RouterBuilderDiscoverExt.html) collect it. A page with a module-derived path is registered by [`module_router!`](macro.module_router.html).

# Handler signature

The function must be `async` and return a [`Result`](../type.Result.html) of a value that implements [`View`](../view/trait.View.html). It can take the request context as [`cx: &Cx`](../context/struct.Cx.html), one request body parameter whose type implements [`FromRequest`](request/trait.FromRequest.html), both, or neither. The body parameter can use a pattern such as `Json(input): Json<T>`. The two parameters can come in either order. Only one body parameter is allowed, because the request body is a stream that can be read only once.

If the body fails to parse, the page fails with the extractor's error, such as `400 Bad Request`, just like an error returned from the function body.

# Examples

An absolute path:

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page("/users/{id}")]
async fn user_profile() -> Result<impl View> {
    Ok(view! { <h1>"User profile"</h1> })
}
```

A module-derived path. In `src/app/about.rs` under `module_router!()`, this serves `/about`:

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page]
async fn about() -> Result<impl View> {
    Ok(view! { <h1>"About"</h1> })
}
```

A path below the module. In `src/app/settings.rs` under `module_router!()`, this serves `/settings/export`:

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page("./export")]
async fn export() -> Result<impl View> {
    Ok(view! { <h1>"Export"</h1> })
}
```

A page for another method, here a form submission answered with a view:

```rust
# use topcoat::{Result, router::{content::Form, page}, view::{View, view}};
# use serde::Deserialize;
# #[derive(Deserialize)]
# struct Signup { email: String }
#[page(POST "/signup")]
async fn signup(Form(input): Form<Signup>) -> Result<impl View> {
    Ok(view! { <h1>"Welcome, " (input.email)</h1> })
}
```

A `GET` page that reads a form. For `GET` and `HEAD` requests, [`Form`](content/struct.Form.html) reads the query string instead of the body:

```rust
# use topcoat::{Result, router::{content::Form, page}, view::{View, view}};
# use serde::Deserialize;
# #[derive(Deserialize)]
# struct Search { q: String }
#[page("/search")]
async fn search(Form(input): Form<Search>) -> Result<impl View> {
    Ok(view! { <main>"searching for " (input.q)</main> })
}
```

# Pages as components

A page is also a [component](../view/attr.component.html). Calling it inside [`view!`](../view/macro.view.html) renders it in place. A page with a body parameter takes the already parsed value as its `body` prop and does not read the request.

```rust
# use topcoat::{Result, router::{content::Form, page}, view::{View, view}};
# use serde::Deserialize;
# #[derive(Deserialize)]
# struct Search { q: String }
# #[page("/search")]
# async fn search(Form(input): Form<Search>) -> Result<impl View> {
#     Ok(view! { <main>"searching for " (input.q)</main> })
# }
#[page("/preview")]
async fn preview() -> Result<impl View> {
    let query = Search {
        q: String::from("topcoat"),
    };
    Ok(view! {
        search(body: Form(query))
    })
}
```
