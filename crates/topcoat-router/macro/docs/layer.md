Declares a layer that wraps request handling under its path.

A layer wraps handlers whose registered path starts with its path. Set an absolute path with `#[layer("/admin")]`. Under [`module_router!`](macro.module_router.html), omit the path to derive it from the enclosing module, or use `./` to extend that path. For example, `#[layer("./v1")]` in `src/app/api.rs` wraps handlers under `/api/v1`.

For a matched handler, the prefix is checked when the router is built, comparing the layer's path to the handler's registered path segment by segment; the request URL is not consulted. A handler is wrapped only when its leading segments spell out the layer's path exactly: a layer at `/docs/admin` wraps neither a page at `/docs/{something}` nor one at `/docs/{*path}`, even though both serve URLs under `/docs/admin`. A parameter segment only matches a parameter of the same name, and group segments count, so a layer at `/dashboard` does not wrap a page at `/(auth)/dashboard` although that page is served at `/dashboard`.

Layers declared with this attribute have a path and wrap only the handlers they match: a 404 (path matched nothing) or 405 (path matched, method did not) never runs them. A layer that returns `None` from [`Layer::path`](trait.Layer.html#tymethod.path) wraps every request instead, misses included, and receives a miss as the `Err` returned by `next.run`. Declare one by implementing the [`Layer`](trait.Layer.html) trait or constructing a [`LayerFn`](struct.LayerFn.html) with a `None` path, registered with [`RouterBuilder::layer`](struct.RouterBuilder.html#method.layer).

When several layers wrap a handler, they nest from least specific (outermost) to most specific (innermost). Layers registered explicitly with [`RouterBuilder::layer`](struct.RouterBuilder.html#method.layer) on the same path nest by registration order, the last registered outermost. Layers collected by [`discover`](trait.RouterBuilderDiscoverExt.html) must have unique paths, because their collection order is not stable.

# Handler signature

The function must be `async` and take [`cx: &Cx`](../context/struct.Cx.html), [`body: Body`](struct.Body.html), and [`next: Next<'_>`](struct.Next.html). It returns `Result<T>`, where `T` implements [`AsyncIntoResponse`](response/trait.AsyncIntoResponse.html). Every [`IntoResponse`](response/trait.IntoResponse.html) type meets this bound. Call [`next.run(cx, body)`](struct.Next.html#method.run) to run the remaining layers and handler. Return directly to answer the request without running them.

# Examples

Explicit path:

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

Module-derived path (in `src/app/api.rs` under `module_router!()`, this wraps every request under `/api`):

```rust
# use topcoat::{Result, context::Cx, router::{Body, Next, layer, response::Response}};
#[layer]
async fn api_log(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}
```

Path below the module (in `src/app/api.rs` under `module_router!()`, this wraps every request under `/api/v1`):

```rust
# use topcoat::{Result, context::Cx, router::{Body, Next, layer, response::Response}};
#[layer("./v1")]
async fn v1_log(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("v1 response: {}", response.status());
    Ok(response)
}
```
