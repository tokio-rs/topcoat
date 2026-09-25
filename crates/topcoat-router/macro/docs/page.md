Declares a page handler.

Pass an absolute path, such as `#[page("/about")]`, to choose the URL directly. Under [`module_router!`](macro.module_router.html), omit the path to derive it from the enclosing module. A path starting with `./` extends that module path. For example, `#[page("./export")]` in `src/app/settings.rs` serves `/settings/export`.

A page serves `GET` by default. To serve other methods, name them before the path, using the same forms as [`#[route]`](attr.route.html): a single method (`#[page(POST "/signup")]`), a bracketed list (`[GET, POST]`), or `*` for every method.

Path strings follow the [`Path`](struct.Path.html) syntax.

A page registers like any other handler: pass the function name to [`RouterBuilder::page`](struct.RouterBuilder.html#method.page), or let [`discover`](trait.RouterBuilderDiscoverExt.html) or [`module_router!`](macro.module_router.html) collect it automatically.

# Handler signature

The function must be `async` and return a [`Result`](../type.Result.html) containing a [`View`](../view/trait.View.html). It may take [`cx: &Cx`](../context/struct.Cx.html) and one body parameter implementing [`FromRequest`](request/trait.FromRequest.html). Both are optional and may appear in either order. The body parameter may use a pattern such as `Json(input): Json<T>`.

# Examples

Explicit path:

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page("/users/{id}")]
async fn user_profile() -> Result<impl View> {
    Ok(view! { <h1>"User profile"</h1> })
}
```

Module-derived path (in `src/app/about.rs` under `module_router!()`, this serves `/about`):

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page]
async fn about() -> Result<impl View> {
    Ok(view! { <h1>"About"</h1> })
}
```

Path below the module (in `src/app/settings.rs` under `module_router!()`, this serves `/settings/export`):

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page("./export")]
async fn export() -> Result<impl View> {
    Ok(view! { <h1>"Export"</h1> })
}
```

Declaring a method (a form submission answered with a rendered view):

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

Reading a request body:

```rust
# use topcoat::{Result, router::{content::Form, page}, view::{View, view}};
# use serde::Deserialize;
# #[derive(Deserialize)]
# struct Search { q: String }
#[page("/contact")]
async fn contact(Form(input): Form<Search>) -> Result<impl View> {
    Ok(view! { <main>"searching for " (input.q)</main> })
}
```

# Pages as components

A page doubles as a [component](../view/attr.component.html): calling it inside [`view!`](../view/macro.view.html) renders it inline. A page that reads a request body takes the already-parsed value as a `body` prop instead of parsing the request.

```rust
# use topcoat::{Result, router::{content::Form, page}, view::{View, view}};
# use serde::Deserialize;
# #[derive(Deserialize)]
# struct Search { q: String }
# #[page("/contact")]
# async fn contact(Form(input): Form<Search>) -> Result<impl View> {
#     Ok(view! { <main>"searching for " (input.q)</main> })
# }
#[page("/preview")]
async fn preview() -> Result<impl View> {
    let query = Search {
        q: String::from("topcoat"),
    };
    Ok(view! {
        contact(body: Form(query))
    })
}
```
