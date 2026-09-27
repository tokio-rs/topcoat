Declares a page handler.

Use `#[page]` without a path string with [module routing](macro.module_router.html). The enclosing module determines the URL. A path starting with `./` extends the module path. An absolute path, such as `#[page("/about")]`, chooses the URL independently of the module tree and requires separate registration.

A page serves `GET` by default. To serve other methods, name them in the attribute, using the same forms as [`#[route]`](attr.route.html): a single method (`#[page(POST)]`), a bracketed list (`[GET, POST]`), or `*` for every method.

Path strings follow the [`Path`](struct.Path.html) syntax.

[`module_router!`](macro.module_router.html) registers module-derived handlers. For explicit paths, pass the function name to [`RouterBuilder::page`](struct.RouterBuilder.html#method.page) or use [`discover`](trait.RouterBuilderDiscoverExt.html).

# Handler signature

The function must be `async` and return a [`Result`](../type.Result.html) containing a [`View`](../view/trait.View.html). It may take [`cx: &Cx`](../context/struct.Cx.html) and one body parameter implementing [`FromRequest`](request/trait.FromRequest.html). Both are optional and may appear in either order. The body parameter may use a pattern such as `Json(input): Json<T>`.

# Examples

Module-derived path (in `src/app/about.rs` under `module_router!()`, this serves `/about`):

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page]
async fn about() -> Result<impl View> {
    Ok(view! { <h1>"About"</h1> })
}
```

Explicit path:

```rust
# use topcoat::{Result, router::page, view::{View, view}};
#[page("/users/{id}")]
async fn user_profile() -> Result<impl View> {
    Ok(view! { <h1>"User profile"</h1> })
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
// src/app/signup.rs
# use topcoat::{Result, router::{content::Form, page}, view::{View, view}};
# use serde::Deserialize;
# #[derive(Deserialize)]
# struct Signup { email: String }
#[page(POST)]
async fn signup(Form(input): Form<Signup>) -> Result<impl View> {
    Ok(view! { <h1>"Welcome, " (input.email)</h1> })
}
```

Reading a request body:

```rust
// src/app/contact.rs
# use topcoat::{Result, router::{content::Form, page}, view::{View, view}};
# use serde::Deserialize;
# #[derive(Deserialize)]
# struct Search { q: String }
#[page]
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
# mod contact {
# use super::*;
# #[page]
# pub(super) async fn contact(Form(input): Form<Search>) -> Result<impl View> {
#     Ok(view! { <main>"searching for " (input.q)</main> })
# }
# }
// src/app/preview.rs
#[page]
async fn preview() -> Result<impl View> {
    let query = Search {
        q: String::from("topcoat"),
    };
    Ok(view! {
        contact::contact(body: Form(query))
    })
}
# fn main() {}
```
