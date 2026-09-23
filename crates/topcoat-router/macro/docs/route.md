Declares an API route handler.

A route always declares its HTTP methods as the first argument:

- a single method, named as it is on `Method` (`GET`, `POST`, and so on),
- a bracketed list (`[GET, POST]`) responding to each listed method, or
- `*`, responding to every method. A route declaring a specific method takes precedence over a `*` route at the same path.

Place an absolute path after the methods to choose the URL directly, as in `#[route(GET "/api/health")]`. Under [`module_router!`](macro.module_router.html), omit the path to derive it from the enclosing module. A path starting with `./` extends the module path. For example, `#[route(GET "./health")]` in `src/app/api.rs` serves `/api/health`.

A route registers like any other handler: pass the function name to [`RouterBuilder::route`](struct.RouterBuilder.html#method.route), or let [`discover`](trait.RouterBuilderDiscoverExt.html) or [`module_router!`](macro.module_router.html) collect it automatically.

# Handler signature

The function must be `async` and return `Result<T>`, where `T` implements [`AsyncIntoResponse`](response/trait.AsyncIntoResponse.html). Every [`IntoResponse`](response/trait.IntoResponse.html) type meets this bound. The handler may take [`cx: &Cx`](../context/struct.Cx.html) and one body parameter implementing [`FromRequest`](request/trait.FromRequest.html). Both are optional and may appear in either order. The body parameter may use a pattern such as `Json(input): Json<T>`.

# Response conversion

The macro converts the success value into a response. See [`IntoResponse`](response/trait.IntoResponse.html) for supported types and tuples. Wrap a value in [`Json<T>`](content/struct.Json.html) to serialize it as JSON.

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
