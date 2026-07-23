Running tower services inside a topcoat router.

The [tower](https://docs.rs/tower) ecosystem shares one service abstraction across axum, hyper, and a large catalog of middleware. This module (behind the `tower` feature) bridges it in the two directions a router cares about: [`TowerRoute`] mounts a tower service as a route, and [`TowerLayer`] runs tower middleware as a layer.

# Mounting a service as a route

[`TowerRoute`] forwards its requests to a tower service (an axum router, a hyper service, a reverse proxy). Registered at a catch-all path with `Methods::Any`, it hands an entire URL subtree to the service. This is the typical setup when migrating an existing application to topcoat one route at a time. The service receives each request with its original URI; nothing is stripped or rewritten.

```rust,ignore
use topcoat::router::{Methods, Path, Router, tower::TowerRoute};

// The pre-migration application, still serving everything under `/legacy`.
let legacy: axum::Router = legacy_app();

let router = Router::builder()
    .route(TowerRoute::new(
        Methods::Any,
        Path::new("/legacy/{*rest}"),
        legacy,
    ))
    .build();
```

A catch-all segment does not match the bare prefix itself, so register a second `TowerRoute` for `/legacy` if the service also serves that URL.

# Running middleware as a layer

[`TowerLayer`] wraps the routes under its path in the middleware a `tower::Layer` builds (a timeout, a rate limit, CORS, compression) and registers like any other layer:

```rust
use std::time::Duration;

use topcoat::router::{Path, Router, tower::TowerLayer};
use tower::timeout::TimeoutLayer;

let router = Router::builder()
    .layer(TowerLayer::new(
        Path::new("/api"),
        TimeoutLayer::new(Duration::from_secs(5)),
    ))
    .build();
```

# Errors

An error produced by wrapped topcoat routes (a 404, a handler error) passes through a [`TowerLayer`]'s middleware and leaves it as the original error value, so outer layers and layouts can still catch it by type. An error a tower service produces itself (middleware timing out, a mounted service failing) surfaces as a [`TowerServiceError`]; unmapped, the router renders it as a 500.

# Requirements

A mounted or wrapping service must be `Clone`, `Send`, and `Sync`; wrap a service that is not `Sync` in `tower::buffer`. See [`TowerRoute`] and [`TowerLayer`] for the exact bounds and the remaining caveats (like middleware that retries).
