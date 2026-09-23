Declares an API route handler.

A route always declares its HTTP methods as the first argument:

- a single method, named as it is on `Method` (`GET`, `POST`, and so on),
- a bracketed list (`[GET, POST]`) responding to each listed method, or
- `*`, responding to every method. A route declaring a specific method takes precedence over a `*` route at the same path.

An optional path string follows the methods (`#[route(GET "/api/health")]`); when omitted, the URL is derived from the function's enclosing module path, kebab-cased, provided the function is reachable from a [`module_router!`](macro.module_router.html). A path starting with `./` is joined onto that module-derived path: `#[route(GET "./health")]` in `src/app/api.rs` serves `/api/health`. All forms register into the same router and can be mixed.

A route registers like any other handler: pass the function name to [`RouterBuilder::route`](struct.RouterBuilder.html#method.route), or let [`discover`](trait.RouterBuilderDiscoverExt.html) or [`module_router!`](macro.module_router.html) collect it automatically.

# Handler signature

The function is `async` and returns `Result<T>` where `T` implements [`AsyncIntoResponse`](response/trait.AsyncIntoResponse.html), which every [`IntoResponse`](response/trait.IntoResponse.html) type does. It may take [`cx: &Cx`](../context/struct.Cx.html), one request body parameter implementing [`FromRequest`](request/trait.FromRequest.html), both, or neither. The body parameter may use a destructuring pattern such as `Json(input): Json<T>`, and the parameters may appear in either order.

# Response conversion

The return type determines the response format. For a JSON response, return [`Json<T>`](content/struct.Json.html). See [`content`](content/index.html) for request and response types.

# Examples

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

Module-derived path (in `src/app/api/health.rs` under `module_router!()`, this serves `GET /api/health`):

```rust
# use topcoat::{Result, router::route};
#[route(GET)]
async fn health() -> Result<&'static str> {
    Ok("ok")
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
#[route([GET, POST] "/form")]
async fn form() -> Result<&'static str> {
    Ok("form")
}

#[route(* "/webhook")]
async fn webhook() -> Result<&'static str> {
    Ok("received")
}
```
