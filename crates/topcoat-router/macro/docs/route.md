Declares an API route handler.

A route always declares its HTTP methods as the first argument:

- a single method, named as it is on `Method` (`GET`, `POST`, and so on),
- a bracketed list (`[GET, POST]`) responding to each listed method, or
- `*`, responding to every method. A route declaring a specific method takes precedence over a `*` route at the same path.

Use `#[route(GET)]` without a path string with [module routing](macro.module_router.html). The enclosing module determines the URL. A path starting with `./` extends the module path. An absolute path, such as `#[route(GET "/api/health")]`, chooses the URL independently of the module tree and requires separate registration.

[`module_router!`](macro.module_router.html) registers module-derived handlers. For explicit paths, pass the function name to [`RouterBuilder::route`](struct.RouterBuilder.html#method.route) or use [`discover`](trait.RouterBuilderDiscoverExt.html).

# Handler signature

The function must be `async` and return `Result<T>`, where `T` implements [`AsyncIntoResponse`](response/trait.AsyncIntoResponse.html). Every [`IntoResponse`](response/trait.IntoResponse.html) type meets this bound. The handler may take [`cx: &Cx`](../context/struct.Cx.html) and one body parameter implementing [`FromRequest`](request/trait.FromRequest.html). Both are optional and may appear in either order. The body parameter may use a pattern such as `Json(input): Json<T>`.

# Response conversion

The macro converts the success value into a response. See [`IntoResponse`](response/trait.IntoResponse.html) for supported types and tuples. Wrap a value in [`Json<T>`](content/struct.Json.html) to serialize it as JSON.

# Examples

Module-derived path (in `src/app/api/health.rs` under `module_router!()`, this serves `GET /api/health`):

```rust
# use topcoat::{Result, router::route};
#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

Explicit method and path, reading a JSON body and answering with one:

```rust
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    router::{content::Json, route},
};

#[derive(Deserialize, Serialize)]
struct CreateUser {
    name: String,
}

#[route(POST "/api/users")]
async fn create_user(Json(input): Json<CreateUser>) -> Result<Json<CreateUser>> {
    Ok(Json(CreateUser { name: input.name }))
}
```

Path below the module (in `src/app/api.rs` under `module_router!()`, this serves `GET /api/health` without a module for it):

```rust
# use topcoat::{Result, router::route};
#[route(GET "./health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

A method list, and a `*` route answering every method (say, a webhook endpoint probed with both `GET` and `POST`):

```rust
# use topcoat::{Result, router::route};
mod form {
    use super::*;

    #[route([GET, POST])]
    async fn form() -> Result<&'static str> {
        Ok("form")
    }
}

mod webhook {
    use super::*;

    #[route(*)]
    async fn webhook() -> Result<&'static str> {
        Ok("received")
    }
}
```
