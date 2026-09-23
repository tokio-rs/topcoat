Declares an API route: a handler that returns a response instead of a view.

The first argument of the attribute is always the HTTP methods the route serves:

- one method, named as on `Method` (`GET`, `POST`, and so on),
- a bracketed list (`[GET, POST]`) to serve each listed method, or
- `*` to serve every method. A route for a specific method wins over a `*` route at the same path.

An optional path string follows the methods. With an absolute path, such as `#[route(GET "/api/health")]`, the route is served at that path. Without a path, the route is served at the path of its module, as described in [`module_router!`](macro.module_router.html). A path that starts with `./` is added to the end of the module path, so `#[route(GET "./health")]` in `src/app/api.rs` serves `/api/health`. One application can mix routes with absolute and module-derived paths.

Register a route with an absolute path by passing its name to [`RouterBuilder::route`](struct.RouterBuilder.html#method.route), or let [`discover`](trait.RouterBuilderDiscoverExt.html) collect it. A route with a module-derived path is registered by [`module_router!`](macro.module_router.html).

# Handler signature

The function must be `async` and return `Result<T>`, where `T` implements [`AsyncIntoResponse`](response/trait.AsyncIntoResponse.html). Every [`IntoResponse`](response/trait.IntoResponse.html) type implements it. The function can take the request context as [`cx: &Cx`](../context/struct.Cx.html), one request body parameter whose type implements [`FromRequest`](request/trait.FromRequest.html), both, or neither. The body parameter can use a pattern such as `Json(input): Json<T>`. The two parameters can come in either order.

# Response conversion

The returned value is turned into a response with [`AsyncIntoResponse::async_into_response`](response/trait.AsyncIntoResponse.html#tymethod.async_into_response). Strings, status codes, byte buffers, `(headers, body)` tuples, and [`Json<T>`](content/struct.Json.html) all work. A value is never serialized to JSON on its own. Wrap it in [`Json<T>`](content/struct.Json.html) to send JSON.

# Examples

An absolute path, reading a JSON body and answering with JSON:

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

A module-derived path. In `src/app/api/health.rs` under `module_router!()`, this serves `GET /api/health`:

```rust
# use topcoat::{Result, router::route};
#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

A path below the module. In `src/app/api.rs` under `module_router!()`, this serves `GET /api/health` without a module of its own:

```rust
# use topcoat::{Result, router::route};
#[route(GET "./health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}
```

A list of methods, and a `*` route that serves every method, such as a webhook that is called with both `GET` and `POST`:

```rust
# use topcoat::{Result, router::route};
#[route([GET, POST] "/form")]
async fn form() -> Result<&'static str> {
    Ok("form")
}

#[route(* "/webhook")]
async fn webhook() -> Result<&'static str> {
    Ok("received")
}
```
