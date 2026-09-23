Use [tower](https://docs.rs/tower) services and middleware with Topcoat.

This module needs the `tower` feature. It has three adapters. [`TowerRoute`] mounts a tower service as a route. [`TowerLayer`] runs tower middleware around Topcoat routes. [`TowerService`] serves a Topcoat router inside another tower application.

# Mounting a service as a route

[`TowerRoute`] forwards matching requests to a tower service, such as an existing axum application. Create it with [`any`](TowerRoute::any) to forward every HTTP method. With a catch-all path, the service handles everything below a URL prefix. This helps when you move an application to Topcoat one route at a time.

```rust
use std::convert::Infallible;

use topcoat::router::{Body, Router, request::Request, response::Response, tower::TowerRoute};
use tower::service_fn;

// Stands in for the existing application, such as an axum router, which
// still serves everything under `/legacy`.
let legacy = service_fn(|_request: Request| async {
    Ok::<_, Infallible>(Response::new(Body::from("legacy")))
});

let router = Router::builder()
    .route(TowerRoute::any("/legacy/{*rest}", legacy))
    .build();
```

The service receives the full request URI, including `/legacy`. A catch-all does not match `/legacy` itself, so register a second `TowerRoute` for that path if the service handles it. Use [`new`](TowerRoute::new) to forward only some HTTP methods.

If the service expects paths relative to where it is mounted, use [`StripPrefixLayer`](crate::StripPrefixLayer). For a `TowerRoute` at `/legacy/{*rest}`, registering `.layer(StripPrefixLayer::new("/legacy"))` makes the service receive `/users/7?page=2` for a request to `/legacy/users/7?page=2`.

# Running middleware as a layer

[`TowerLayer`] runs tower middleware as part of Topcoat's request handling. By default it wraps every request, including requests that match no route. Use [`at`](TowerLayer::at) to apply it only to the matched routes under a path:

```rust,no_run
use std::time::Duration;

use topcoat::router::{Router, tower::TowerLayer};
use tower::timeout::TimeoutLayer;

let router = Router::builder()
    .layer(TowerLayer::new(TimeoutLayer::new(Duration::from_secs(5))).at("/api"))
    .build();
```

# Serving a Topcoat router in another application

[`TowerService`] wraps a Topcoat [`Router`](crate::Router) in a tower service. Use it when another application runs the HTTP server. For example, an axum application can pass the requests it does not handle to Topcoat:

```rust,ignore
use topcoat::router::{Router, RouterBuilderDiscoverExt, tower::TowerService};

let topcoat = Router::builder().discover().build();

// Serve every request that the axum application does not handle.
let app = axum::Router::new()
    .route("/api/health", axum::routing::get(|| async { "ok" }))
    .fallback_service(TowerService::new(topcoat));
```

Pass the full request path to Topcoat, as the root-level fallback above does. Topcoat matches that path against its routes and builds URLs from its own route paths. If the service is mounted behind something that strips a path prefix, the links Topcoat generates can point outside the mount.

The service accepts any request body that yields bytes. It never returns a service error, because it turns routing errors and handler panics into HTTP responses.

# Passing the connection address

When another server accepts the connections, that server must tell Topcoat the peer address. Insert a [`RemoteAddr`](crate::RemoteAddr) into the extensions of each request before passing it to [`TowerService`]. Use the address reported by the connection. Behind a reverse proxy, this is the address of the proxy.

In axum, a middleware can copy the address from the request's `ConnectInfo<SocketAddr>` extension:

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

To fill in that extension, serve the axum application with `app.into_make_service_with_connect_info::<SocketAddr>()`. See axum's [`ConnectInfo` documentation](https://docs.rs/axum/latest/axum/extract/struct.ConnectInfo.html) for the server setup.

Topcoat's [`remote_addr`](crate::request::remote_addr) then returns the connection address, and [`client_ip`](crate::request::client_ip) uses its IP by default. If requests reach the server through a reverse proxy, configure [`TrustedProxies`](crate::TrustedProxies) on the Topcoat router to read the client's IP from the proxy's header.

Without a `RemoteAddr`, `remote_addr(cx)` returns `None`. You can still trust proxies by their position with [`TrustedProxies::nearest`](crate::TrustedProxies::nearest), for example a proxy connected over a Unix socket. The direct connection counts as the first hop even when its address is unknown. Only do this when every request passes through the configured number of proxies. Running Topcoat inside another tower application does not add a proxy hop by itself.

# Errors

A Topcoat error that passes through a [`TowerLayer`] keeps its original type, so outer layers and layouts can still catch it. Errors returned by the tower middleware or by a mounted service become a [`TowerServiceError`]. Unless the application handles them, Topcoat responds to them with `500 Internal Server Error`.

# Requirements

Services used with [`TowerRoute`] or [`TowerLayer`] must be `Clone`, `Send`, and `Sync`. A service that is not `Sync` can be wrapped in `tower::buffer`. `TowerLayer` does not support middleware that calls its inner service more than once per request, such as retry middleware. See the documentation of each adapter for the full trait bounds.
