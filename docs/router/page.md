Declares a page handler.

Use `#[page]` without a path string with [module routing](macro.module_router.html). The enclosing module determines the URL. A path starting with `./` extends the module path. An absolute path, such as `#[page("/about")]`, chooses the URL independently of the module tree and requires separate registration.

A page serves `GET` by default. To serve other methods, name them in the attribute, using the same forms as [`#[route]`](attr.route.html): a single method (`#[page(POST)]`), a bracketed list (`[GET, POST]`), or `*` for every method.

Path strings follow the [`Path`](struct.Path.html) syntax.

[`module_router!`](macro.module_router.html) registers module-derived handlers. For explicit paths, pass the function name to [`RouterBuilder::page`](struct.RouterBuilder.html#method.page) or use [`discover`](trait.RouterBuilderDiscoverExt.html).

# Handler signature

The function must be `async` and return a [`Result`](../type.Result.html) containing a [`View`](../view/trait.View.html). It may take [`cx: &Cx`](../context/struct.Cx.html) and one body parameter implementing [`FromRequest`](request/trait.FromRequest.html). Both are optional and may appear in either order. The body parameter may use a pattern such as `Json(input): Json<T>`.

# Generic pages

Pages can have type and const parameters. Choose their arguments when explicitly registering the generated marker with `.page(home::<MyState>::default())`. The page can depend on a trait rather than the concrete state type chosen by the application:

```rust
use topcoat::{
    Result,
    context::{Cx, context},
    router::{Router, href, page},
    view::{View, view},
};

trait SiteState: Send + Sync + 'static {
    fn title(&self) -> &str;
}

#[page("/")]
async fn home<T: SiteState>(cx: &Cx) -> Result<impl View> {
    let state = context::<T>(cx);
    Ok(view! {
        <h1>(state.title())</h1>
        <a href=(href!(home::<T>::default()))>"Home"</a>
    })
}

fn site_router<T: SiteState>(state: T) -> Router {
    Router::builder()
        .with(state)
        .page(home::<T>::default())
        .build()
}
```

The marker's `Default` implementation does not require the state type to implement `Default`. Constructing it does not create any state; `.with(state)` supplies the value that `context::<T>(cx)` reads. The context type must exactly match the registered value, and a missing value still causes a runtime panic.

Bounds in a `where` clause are preserved. Type parameters may appear only in the function body, in the request body type, or in route-context expressions such as `with(Title(T::TITLE))`. The specialized page marker must be `Send + Sync + 'static`. Use elided lifetimes for request borrows; page functions cannot declare lifetime parameters.

Each type/const specialization has its own stable route identity. Use a concrete marker value with `href!`, as above, to link to that specialization. Generic pages remain callable inside `view!`, for example `home::<MyState>()`.

Generic pages are **not automatically discovered**, even when `discover` is enabled: discovery cannot choose their generic arguments. Prefer an explicit absolute path and manual registration. Generic module-derived pages implement `ModulePage` and can be registered through `ModuleRouterBuilder`, but `module_router!()` does not collect them automatically. Two specializations registered at the same path and method still conflict like any other pair of pages.

# Route context

Attach metadata with `with(...)`, such as `#[page(with(Title("Products")))]` or `#[page("/products", with(Title("Products")))]`. You can add multiple `with(...)` entries, separated by commas, after any methods and path. The values are initialized when the router is built, so an outer layout can read the page's title before the page runs. See [route context](../context/index.html#route-context) for a complete example.

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
