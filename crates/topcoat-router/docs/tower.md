Running tower services inside a topcoat router.

The [tower](https://docs.rs/tower) ecosystem shares one service abstraction across axum, hyper, and a large catalog of middleware. This module (behind the `tower` feature) bridges it in both directions: [`TowerRoute`] mounts a tower service as a route and [`TowerLayer`] runs tower middleware as a layer inside a topcoat router, while [`TowerService`] serves a whole topcoat router as a tower service.

# Mounting a service as a route

[`TowerRoute`] forwards its requests to a tower service (an axum router, a hyper service, a reverse proxy). Registered with [`any`](TowerRoute::any) at a catch-all path, it hands an entire URL subtree to the service, which responds to every HTTP method. This is the typical setup when migrating an existing application to topcoat one route at a time. The service receives each request with its original URI; nothing is stripped or rewritten.

```rust,ignore
use topcoat::router::{Router, tower::TowerRoute};

// The pre-migration application, still serving everything under `/legacy`.
let legacy: axum::Router = legacy_app();

let router = Router::builder()
    .route(TowerRoute::any("/legacy/{*rest}", legacy))
    .build();
```

A catch-all segment does not match the bare prefix itself, so register a second `TowerRoute` for `/legacy` if the service also serves that URL. To restrict a mounted service to specific methods, use [`new`](TowerRoute::new) instead.

# Running middleware as a layer

[`TowerLayer`] wraps routes in the middleware a `tower::Layer` builds (a timeout, a rate limit, CORS, compression) and registers like any other layer. It wraps every route by default; scope it to the routes under a path prefix with [`at`](TowerLayer::at):

```rust,no_run
use std::time::Duration;

use topcoat::router::{Router, tower::TowerLayer};
use tower::timeout::TimeoutLayer;

let router = Router::builder()
    .layer(TowerLayer::new(TimeoutLayer::new(Duration::from_secs(5))).at("/api"))
    .build();
```

# Mounting a topcoat router in a tower application

[`TowerService`] turns the bridge around: it serves a whole topcoat [`Router`](crate::Router) as a tower service, so an application that owns the HTTP server (an axum router, a hyper server) can embed a topcoat application. The service accepts a request with any body and never errors; the router renders every failure, including a handler panic, as a response.

```rust,ignore
use topcoat::router::{Router, RouterBuilderDiscoverExt, tower::TowerService};

let topcoat = Router::builder().discover().build();

// Serve every request the surrounding axum application does not handle.
let app = axum::Router::new()
    .route("/api/health", axum::routing::get(|| async { "ok" }))
    .fallback_service(TowerService::new(topcoat));
```

# Errors

An error produced by wrapped topcoat routes (a 404, a handler error) passes through a [`TowerLayer`]'s middleware and leaves it as the original error value, so outer layers and layouts can still catch it by type. An error a tower service produces itself (middleware timing out, a mounted service failing) surfaces as a [`TowerServiceError`]; unmapped, the router renders it as a 500.

# Requirements

A mounted or wrapping service must be `Clone`, `Send`, and `Sync`; wrap a service that is not `Sync` in `tower::buffer`. See [`TowerRoute`] and [`TowerLayer`] for the exact bounds and the remaining caveats (like middleware that retries).
