Use tower services and middleware with Topcoat.

Enable the `tower` feature to connect Topcoat with [tower](https://docs.rs/tower). Mount an existing service with [`TowerRoute`], apply middleware with [`TowerLayer`], or expose a Topcoat router through [`TowerService`].

# Mounting a service as a route

[`TowerRoute`] forwards requests to a tower service. Use [`any`](TowerRoute::any) to accept every HTTP method and a catch-all path to forward a URL subtree. For example, an existing axum application can keep serving `/legacy` while other routes move to Topcoat:

```rust,ignore
use topcoat::router::{Router, tower::TowerRoute};

// The pre-migration application, still serving everything under `/legacy`.
let legacy: axum::Router = legacy_app();

let router = Router::builder()
    .route(TowerRoute::any("/legacy/{*rest}", legacy))
    .build();
```

The service receives the full URI, including `/legacy`. The catch-all requires another segment, so register `/legacy` separately if the service should handle that URL too. Use [`new`](TowerRoute::new) to restrict the HTTP methods.

Add [`StripPrefixLayer`](crate::StripPrefixLayer) if the service expects paths relative to its mount point. With `.layer(StripPrefixLayer::new("/legacy"))`, a request for `/legacy/users/7?page=2` reaches the mounted service as `/users/7?page=2`.

# Running middleware as a layer

[`TowerLayer`] runs tower middleware around Topcoat request handling. By default, it covers every request, including unmatched requests. Use [`at`](TowerLayer::at) to limit it to matched handlers under a path:

```rust,no_run
use std::time::Duration;

use topcoat::router::{Router, tower::TowerLayer};
use tower::timeout::TimeoutLayer;

let router = Router::builder()
    .layer(TowerLayer::new(TimeoutLayer::new(Duration::from_secs(5))).at("/api"))
    .build();
```

# Serving a Topcoat router in another application

[`TowerService`] lets another application serve a Topcoat [`Router`](crate::Router). For example, an axum application can forward otherwise unmatched requests to Topcoat:

```rust,ignore
use topcoat::router::{Router, RouterBuilderDiscoverExt, tower::TowerService};

let topcoat = Router::builder().discover().build();

// Serve every request the surrounding axum application does not handle.
let app = axum::Router::new()
    .route("/api/health", axum::routing::get(|| async { "ok" }))
    .fallback_service(TowerService::new(topcoat));
```

Forward the full path, as the fallback above does. Topcoat uses its registered paths for both request matching and URL generation. If the surrounding service strips a prefix, generated links may point outside the mount.

The service accepts compatible byte-stream request bodies. Routing errors and handler panics become HTTP responses. The service itself has no error result to handle.

# Passing the connection address

To expose the connection address, insert a [`RemoteAddr`](crate::RemoteAddr) into each request's extensions before calling [`TowerService`]. Use the transport's peer address, which belongs to the proxy when a reverse proxy connects to your server.

In axum, middleware can copy the address from the request's `ConnectInfo<SocketAddr>` extension:

```rust,ignore
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Request};
use topcoat::router::RemoteAddr;

let app = app.layer(axum::middleware::map_request(|mut request: Request| async {
    if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        let addr = *addr;
        request.extensions_mut().insert(RemoteAddr(addr));
    }
    request
}));
```

Serve the axum application with `app.into_make_service_with_connect_info::<SocketAddr>()` to populate that extension. See axum's [`ConnectInfo` documentation](https://docs.rs/axum/latest/axum/extract/struct.ConnectInfo.html) for the server setup.

[`remote_addr`](crate::request::remote_addr) reads this address. [`client_ip`](crate::request::client_ip) uses its IP unless you configure [`TrustedProxies`](crate::TrustedProxies) to read the client's address from a trusted proxy's header.

Without `RemoteAddr`, `remote_addr(cx)` returns `None`. You can still trust a proxy by its position with [`TrustedProxies::nearest`](crate::TrustedProxies::nearest). This supports connections without an IP address, such as Unix sockets. The direct connection counts as the first hop. Use this only when every request passes through the configured number of proxies. Embedding Topcoat in another tower application adds no proxy hop.

# Errors

[`TowerLayer`] preserves errors from the inner Topcoat handler so outer code can catch them by type. Errors created by tower middleware or a mounted service become [`TowerServiceError`]. An unhandled service error produces `500 Internal Server Error`.

# Requirements

Services used with [`TowerRoute`] or [`TowerLayer`] must implement `Clone + Send + Sync`. Wrap a service in `tower::buffer` if it needs to become `Sync`. `TowerLayer` permits only one inner call per request, so retry middleware is unsupported. See each adapter for the full bounds.
