Serving files from a directory as a route.

[`ServeDir`] is a [`Route`](crate::Route) that reads files from a filesystem directory. Register it at a catch-all path with [`RouterBuilder::route`](crate::RouterBuilder::route). It strips the static prefix of its own path, so a file at `res/hello.txt` is served at `/res/hello.txt` when the route is `/res/{*path}` and the directory is `res`.

```rust
use topcoat::router::{Router, ServeDir};

let router = Router::builder()
    .route(ServeDir::new("/res/{*path}", "res"))
    .build();
```

The route responds to `GET` and `HEAD`. A directory request serves `index.html` when that file exists. Percent-encoded path segments are decoded. A path that would leave the directory (`..`, or a NUL byte) is rejected as not found.

This route is available behind the `serve` feature. To mount a tower `ServeDir` instead, use [`TowerRoute`](crate::tower::TowerRoute) with a [`StripPrefix`](crate::StripPrefix) layer.
