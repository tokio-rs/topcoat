Declares a layer: a function that wraps the handling of requests below its path.

A layer wraps every page and API route whose path starts with the layer's path, like a [`#[layout]`](attr.layout.html) does. A layer at `/admin` wraps the handlers under `/admin`, and a layer at `/` wraps all of them. The attribute takes an optional path string. With an absolute path, such as `#[layer("/admin")]`, the layer uses that path. Without a path, the layer uses the path of its module, as described in [`module_router!`](macro.module_router.html). A path that starts with `./` is added to the end of the module path, so `#[layer("./v1")]` in `src/app/api.rs` wraps the handlers under `/api/v1`.

The match is made when the router is built, by comparing the layer's path with each handler's registered path segment by segment. The request URL is not used. A handler is wrapped only when the first segments of its path are exactly the layer's path. For example, a layer at `/docs/admin` wraps neither a page at `/docs/{something}` nor one at `/docs/{*path}`, even though both serve URLs under `/docs/admin`. A parameter segment only matches a parameter with the same name. Group segments count too, so a layer at `/dashboard` does not wrap a page at `/(auth)/dashboard`, even though that page is served at `/dashboard`.

A layer declared with this attribute always has a path, so it only wraps the handlers it matches. It does not run for a 404 (no path matched) or a 405 (the path matched but the method did not). A layer whose [`Layer::path`](trait.Layer.html#tymethod.path) returns `None` wraps every request, including these misses, and receives a miss as the `Err` returned by `next.run`. To declare one, implement the [`Layer`](trait.Layer.html) trait or build a [`LayerFn`](struct.LayerFn.html) with a `None` path, and register it with [`RouterBuilder::layer`](struct.RouterBuilder.html#method.layer).

When several layers wrap a handler, they nest by path. The layer with the shortest path is the outermost, and the one with the longest path is the innermost. Layers registered with [`RouterBuilder::layer`](struct.RouterBuilder.html#method.layer) at the same path nest in registration order, with the last one registered on the outside. Layers collected by [`discover`](trait.RouterBuilderDiscoverExt.html) or [`module_router!`](macro.module_router.html) must each have a different path, because the order in which they are collected is not stable.

# Handler signature

The function must be `async` and take three parameters in this order: [`cx: &Cx`](../context/struct.Cx.html), the request [`body: Body`](struct.Body.html), and [`next: Next<'_>`](struct.Next.html). It returns `Result<T>`, where `T` implements [`AsyncIntoResponse`](response/trait.AsyncIntoResponse.html). Every [`IntoResponse`](response/trait.IntoResponse.html) type implements it.

Call [`next.run(cx, body)`](struct.Next.html#method.run) to run the inner layers and then the handler. A layer that returns without calling `next.run` stops the request there, and its return value becomes the response.

Register a layer with an absolute path by passing its name to [`RouterBuilder::layer`](struct.RouterBuilder.html#method.layer), or let [`discover`](trait.RouterBuilderDiscoverExt.html) collect it. A layer with a module-derived path is registered by [`module_router!`](macro.module_router.html).

# Examples

An absolute path:

```rust
use topcoat::{
    Result,
    context::Cx,
    router::{Body, Next, layer, response::Response},
};

#[layer("/")]
async fn timing(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let start = std::time::Instant::now();
    let response = next.run(cx, body).await?;
    println!("handled in {:?}", start.elapsed());
    Ok(response)
}
```

A module-derived path. In `src/app/api.rs` under `module_router!()`, this wraps every handler under `/api`:

```rust
# use topcoat::{Result, context::Cx, router::{Body, Next, layer, response::Response}};
#[layer]
async fn api_log(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}
```

A path below the module. In `src/app/api.rs` under `module_router!()`, this wraps every handler under `/api/v1`:

```rust
# use topcoat::{Result, context::Cx, router::{Body, Next, layer, response::Response}};
#[layer("./v1")]
async fn v1_log(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("v1 response: {}", response.status());
    Ok(response)
}
```
