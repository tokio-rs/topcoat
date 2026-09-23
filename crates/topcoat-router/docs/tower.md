Use tower services and middleware with Topcoat.

This module is available with the `tower` feature. Use [`TowerRoute`] to mount a [tower](https://docs.rs/tower) service as a route, [`TowerLayer`] to apply tower middleware to Topcoat routes, or [`TowerService`] to serve a Topcoat router inside another tower application.

# Mounting a service as a route

[`TowerRoute`] forwards matching requests to a tower service, such as an existing axum application. Register it with [`any`](TowerRoute::any) to forward every HTTP method. A catch-all path lets the service handle everything below a URL prefix, which is useful when moving an application to Topcoat one route at a time.

```rust
use std::convert::Infallible;
use topcoat::router::{Body, Router, request::Request, response::Response, tower::TowerRoute};

let legacy = tower::service_fn(|_request: Request| async {
    Ok::<_, Infallible>(Response::new(Body::from("legacy")))
});

let router = Router::builder()
    .route(TowerRoute::any("/legacy/{*rest}", legacy))
    .build();
```

The service receives the full request URI, including `/legacy`. A catch-all does not match `/legacy` itself, so register a second `TowerRoute` for that path if the service handles it. Use [`new`](TowerRoute::new) to forward only specific HTTP methods.

Use [`StripPrefixLayer`](crate::StripPrefixLayer) when the service expects paths relative to its mount point. For a `TowerRoute` at `/legacy/{*rest}`, registering `.layer(StripPrefixLayer::new("/legacy"))` makes the service receive `/users/7?page=2` for a request to `/legacy/users/7?page=2`.

# Running middleware as a layer

[`TowerLayer`] applies tower middleware to Topcoat request handling. It wraps every request by default, including requests with no matching route. Use [`at`](TowerLayer::at) to apply it only to matched routes under a path prefix:

```rust,no_run
use std::time::Duration;

use topcoat::router::{Router, tower::TowerLayer};
use tower::timeout::TimeoutLayer;

let router = Router::builder()
    .layer(TowerLayer::new(TimeoutLayer::new(Duration::from_secs(5))).at("/api"))
    .build();
```

# Serving a Topcoat router in another application

[`TowerService`] wraps a Topcoat [`Router`](crate::Router) in a tower service. Pass it to the application that owns your HTTP server:

```rust
use topcoat::router::{Router, tower::TowerService};

let router = Router::builder().build();
let service = TowerService::new(router);
```

Forward the full request path to Topcoat. Topcoat matches that path against its routes and generates URLs from its own route paths. Mounting it behind a service that strips a prefix can make generated links point outside the mount.

The service accepts request bodies that yield bytes and satisfy its trait bounds. It returns routing errors and handler panics as HTTP responses, so callers do not need to handle a service error.

# Passing the connection address

When another server accepts the connections, it must pass the peer address to Topcoat. Insert a [`RemoteAddr`](crate::RemoteAddr) into each request's extensions before passing it to [`TowerService`]. Use the address reported by the transport. Behind a reverse proxy, this is the proxy's address.

Insert the address before forwarding each request:

```rust
use std::net::SocketAddr;
use topcoat::router::{RemoteAddr, request::Request};

fn set_peer(request: &mut Request, peer: SocketAddr) {
    request.extensions_mut().insert(RemoteAddr(peer));
}
```

Topcoat's [`remote_addr`](crate::request::remote_addr) then returns the connection address, and [`client_ip`](crate::request::client_ip) uses its IP by default. If the server receives requests through a reverse proxy, configure [`TrustedProxies`](crate::TrustedProxies) on the Topcoat router to read the client's IP from the proxy's header.

Without `RemoteAddr`, `remote_addr(cx)` returns `None`. The connection can still be trusted by position with [`TrustedProxies::nearest`](crate::TrustedProxies::nearest), as with a proxy connected over a Unix socket. The direct connection counts as the first hop even when its address is unknown. Only use this when every request must pass through the configured number of proxies. Running Topcoat inside another tower application does not itself add a proxy hop.

# Errors

A Topcoat error passing through [`TowerLayer`] keeps its original type, so outer layers and layouts can still catch it. Errors returned by the tower middleware or a mounted service become [`TowerServiceError`]. Unless the application handles them, Topcoat renders them as `500 Internal Server Error` responses.

# Requirements

Services used with [`TowerRoute`] or [`TowerLayer`] must be `Clone`, `Send`, and `Sync`. A service that is not `Sync` can be wrapped in `tower::buffer`. Middleware that calls its inner service more than once per request, such as retry middleware, is not supported by `TowerLayer`. See the adapter documentation for the full trait bounds.
