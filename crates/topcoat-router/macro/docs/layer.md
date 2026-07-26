Declares a layer that wraps request handling under its path.

A layer wraps every page and API route whose path begins with the layer's path, following the same prefix rule as [`#[layout]`](attr.layout.html): a layer at `/admin` wraps the handlers under `/admin`, while a layer at `/` wraps them all. The layer's path is the string given to the attribute (`#[layer("/admin")]`); when omitted, it is derived from the function's enclosing module path, kebab-cased, provided the function is reachable from a [`module_router!`](macro.module_router.html).

For a matched handler, the prefix is checked when the router is built, comparing the layer's path to the handler's registered path segment by segment; the request URL is not consulted. A handler is wrapped only when its leading segments spell out the layer's path exactly: a layer at `/docs/admin` wraps neither a page at `/docs/{something}` nor one at `/docs/{*path}`, even though both serve URLs under `/docs/admin`. A parameter segment only matches a parameter of the same name, and group segments count, so a layer at `/dashboard` does not wrap a page at `/(auth)/dashboard` although that page is served at `/dashboard`.

Only when no path matches at all does the request URL come into play: a 404 response runs the layers whose path matches the request URL, checked at request time. A 405 comes from a matched path and runs that path's layers.

When several layers wrap a handler, they nest from least specific (outermost) to most specific (innermost). Layers registered explicitly with [`RouterBuilder::layer`](struct.RouterBuilder.html#method.layer) on the same path nest by registration order, the last registered outermost. Layers collected by [`discover`](trait.RouterBuilderDiscoverExt.html) must have unique paths, because their collection order is not stable.

# Handler signature

The function is `async` and takes [`cx: &mut CxBuilder`](../context/struct.CxBuilder.html), the request [`body: Body`](struct.Body.html), and a [`next: Next<'_>`](struct.Next.html), returning `Result<T>` where `T` implements [`IntoResponse`](trait.IntoResponse.html). Call [`next.run(cx, body)`](struct.Next.html#method.run) to invoke the inner layers and ultimately the handler. Returning without calling `next.run` short-circuits the request: the layer's return value becomes the response.

# Examples

Explicit path:

```rust
use topcoat::{
    Result,
    context::CxBuilder,
    router::{Body, Next, Response, layer},
};

#[layer("/")]
async fn timing(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    let start = std::time::Instant::now();
    let response = next.run(cx, body).await?;
    println!("handled in {:?}", start.elapsed());
    Ok(response)
}
```

Module-derived path (in `src/app/api.rs` under `module_router!()`, this wraps every request under `/api`):

```rust
# use topcoat::{Result, context::CxBuilder, router::{Body, Next, Response, layer}};
#[layer]
async fn api_log(cx: &mut CxBuilder, body: Body, next: Next<'_>) -> Result<Response> {
    let response = next.run(cx, body).await?;
    println!("API response: {}", response.status());
    Ok(response)
}
```
