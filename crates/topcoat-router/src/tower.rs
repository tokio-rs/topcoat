#![doc = include_str!("../docs/tower.md")]

use std::borrow::Cow;
use std::fmt::{self, Display};
use std::future::Future;
use std::pin::{Pin, pin};
use std::task::{Context, Poll};

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};
use topcoat_core::context::{Cx, CxBuilder};
use topcoat_core::error::{Error, Result};
use tower::ServiceExt;

use crate::{
    Body, BoxError, Layer, LayerFuture, Methods, Next, OwnedMethods, Path, Request, Response,
    Route, RouteFuture, parts,
};

/// A [`Route`] that forwards its requests to a tower service.
///
/// This adapter mounts a whole tower application (an axum router, a hyper
/// service, a reverse proxy) as a route in a topcoat router, typically while
/// migrating an existing application to topcoat one route at a time.
/// Registered at a catch-all path with [`Methods::Any`], it hands an entire
/// URL subtree to the service. The service receives each request with its
/// original URI; nothing is stripped or rewritten. A catch-all segment does
/// not match the bare prefix itself, so register a second `TowerRoute` for
/// the prefix if the service also serves that URL.
///
/// The service must be `Clone`, `Send`, and `Sync`; wrap a service that is
/// not `Sync` in `tower::buffer`. Its per-request clones share cross-request
/// state through the service's internal handles.
///
/// An error the mounted service returns surfaces as an [`Error`] wrapping a
/// [`TowerServiceError`]; unmapped, the router renders it as a 500. Layers
/// wrapping the route's path apply as they would to any other route.
///
/// Register the adapter with
/// [`RouterBuilder::route`](crate::RouterBuilder::route).
///
/// # Examples
///
/// ```rust
/// use std::convert::Infallible;
///
/// use topcoat::router::{Body, Methods, Path, Request, Response, Router, tower::TowerRoute};
/// use tower::service_fn;
///
/// // Stands in for a legacy tower application, like an axum router.
/// let legacy = service_fn(|_request: Request| async {
///     Ok::<_, Infallible>(Response::new(Body::from("legacy")))
/// });
///
/// let router = Router::builder()
///     .route(TowerRoute::new(
///         Methods::Any,
///         Path::new("/legacy/{*rest}"),
///         legacy,
///     ))
///     .build();
/// ```
pub struct TowerRoute<S> {
    /// The HTTP methods this route responds to.
    methods: OwnedMethods,
    /// The URL path this route handles.
    path: Cow<'static, Path>,
    /// The mounted tower service, cloned per request.
    service: S,
}

impl<S> TowerRoute<S> {
    /// Mounts `service` at `path`, responding to `methods`.
    ///
    /// The methods are anything convertible into [`OwnedMethods`]: a single
    /// [`Method`](crate::Method), a `&'static [Method]`, a `Vec<Method>`, or
    /// [`Methods::Any`] to respond to every method. A route registered for a
    /// specific method takes precedence over an any-method route at the same
    /// path.
    #[must_use]
    pub fn new(
        methods: impl Into<OwnedMethods>,
        path: impl Into<Cow<'static, Path>>,
        service: S,
    ) -> Self {
        Self {
            methods: methods.into(),
            path: path.into(),
            service,
        }
    }
}

impl<S, ResBody> Route for TowerRoute<S>
where
    S: tower::Service<Request, Response = http::Response<ResBody>> + Clone + Send + Sync + 'static,
    S::Error: Into<BoxError> + Send,
    S::Future: Send,
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
{
    fn methods(&self) -> Methods<'_> {
        self.methods.as_methods()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        let service = self.service.clone();
        Box::pin(async move {
            // Reassemble the http request the service consumes from a copy of
            // the parts on the context; the originals stay available to outer
            // layers and error rendering.
            let request = Request::from_parts(parts(cx).clone(), body);
            match service.oneshot(request).await {
                Ok(response) => Ok(response.map(Body::new)),
                Err(error) => Err(TowerServiceError(error.into()).into()),
            }
        })
    }
}

/// A [`Layer`] that wraps the routes nested under its path in a
/// [`tower::Layer`]'s middleware.
///
/// This adapter runs middleware from the tower ecosystem (a timeout, a rate
/// limit, CORS, compression) inside a topcoat router. The middleware behaves
/// as it would in a plain tower stack: its state (a concurrency-limit
/// semaphore, a rate-limit window) is shared across requests, and changes it
/// makes to the request are seen by the layers and route it wraps.
///
/// The middleware's service must be `Clone`, `Send`, and `Sync`; wrap a
/// service that is not `Sync` in `tower::buffer`. To run several tower
/// layers, compose them first (for example with [`tower::ServiceBuilder`])
/// and wrap the result in a single `TowerLayer`. Middleware that calls its
/// inner service more than once per request (like `tower::retry`) is not
/// supported.
///
/// An error produced by the wrapped routes (a 404, a handler error) leaves
/// the layer as the original [`Error`] value, while an error produced by the
/// middleware itself (a timeout elapsing, a load-shed rejection) surfaces as
/// an `Err` wrapping a [`TowerServiceError`].
///
/// Register the adapter with
/// [`RouterBuilder::layer`](crate::RouterBuilder::layer).
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
///
/// use topcoat::router::{Path, Router, tower::TowerLayer};
/// use tower::timeout::TimeoutLayer;
///
/// let router = Router::builder()
///     .layer(TowerLayer::new(
///         Path::new("/api"),
///         TimeoutLayer::new(Duration::from_secs(5)),
///     ))
///     .build();
/// ```
pub struct TowerLayer<S> {
    /// The URL path prefix whose routes this layer wraps.
    path: Cow<'static, Path>,
    /// The composed tower service, built once and cloned per request.
    service: S,
}

impl<S> TowerLayer<S> {
    /// Wraps the routes under `path` in the middleware `layer` builds.
    ///
    /// The middleware is built immediately and shared by every request
    /// passing through this layer.
    #[must_use]
    pub fn new<L>(path: impl Into<Cow<'static, Path>>, layer: L) -> Self
    where
        L: tower::Layer<TowerNext, Service = S>,
    {
        Self {
            path: path.into(),
            service: layer.layer(TowerNext::new()),
        }
    }
}

impl<S, ResBody> Layer for TowerLayer<S>
where
    S: tower::Service<Request, Response = http::Response<ResBody>> + Clone + Send + Sync + 'static,
    S::Error: Into<BoxError> + Send,
    S::Future: Send,
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
{
    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        // Clones of a tower service share its cross-request state (semaphores,
        // rate-limit windows) through the service's internal handles.
        let service = self.service.clone();
        Box::pin(async move {
            // Swap out existing parts with empty ones to avoid cloning headers etc.
            let parts = cx
                .insert(http::Request::new(()).into_parts().0)
                .expect("router context contains parts");
            // Reassemble the http request the middleware operates on from the
            // parts stored on the context, and slip it the relay over which
            // `TowerNext` calls back into this chain.
            let mut request = Request::from_parts(parts, body);
            let (relay, mut chain_calls) = relay_channel();
            request.extensions_mut().insert(relay);

            let mut middleware = pin!(service.oneshot(request));

            // Drive the middleware until it either responds on its own or
            // calls through to the wrapped chain.
            let (request, respond_to) = tokio::select! {
                result = &mut middleware => return finish(result),
                called = chain_calls.recv() => match called {
                    Some(call) => call,
                    // The middleware dropped the request without calling the
                    // chain; it produces a response on its own.
                    None => return finish(middleware.await),
                },
            };

            // Run the chain concurrently with the middleware, so middleware
            // racing the chain (like a timeout) stays live and can cancel it.
            let chain = async move {
                let (mut parts, body) = request.into_parts();
                // Write the request back so middleware edits are visible to
                // inner layers and the route.
                parts.extensions.remove::<Relay>();
                cx.insert(parts);
                let result = next.run(cx, body).await.map_err(TowerNextError::tunneled);
                let _ = respond_to.send(result);
                // The chain runs at most once; answer any repeated call.
                while let Some((_, respond_to)) = chain_calls.recv().await {
                    let _ = respond_to.send(Err(TowerNextError::consumed()));
                }
            };
            let mut chain = pin!(chain);
            tokio::select! {
                result = &mut middleware => return finish(result),
                () = &mut chain => {}
            }
            finish(middleware.await)
        })
    }
}

/// The inner service a [`TowerLayer`]'s middleware wraps.
///
/// [`TowerLayer::new`] hands this service to the given [`tower::Layer`].
/// Calling it forwards the request to the layers and route the `TowerLayer`
/// wraps and resolves with their response. It can be called at most once per
/// request; a repeated call (like a retry's) resolves to a
/// [`TowerNextError`].
#[derive(Clone, Debug)]
pub struct TowerNext {
    _priv: (),
}

impl TowerNext {
    /// Creates the stand-in service handed to a [`TowerLayer`]'s middleware.
    fn new() -> Self {
        Self { _priv: () }
    }
}

impl tower::Service<Request> for TowerNext {
    type Response = Response;
    type Error = TowerNextError;
    type Future = Pin<Box<dyn Future<Output = Result<Response, TowerNextError>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        // The relay back to the adapter rides in the request extensions, so it
        // survives whatever transformation the middleware applies.
        let relay = request.extensions_mut().remove::<Relay>();
        Box::pin(async move {
            let Some(Relay(relay)) = relay else {
                return Err(TowerNextError::detached());
            };
            let (respond_to, response) = oneshot::channel();
            if relay.send((request, respond_to)).await.is_err() {
                return Err(TowerNextError::cancelled());
            }
            response
                .await
                .unwrap_or_else(|_| Err(TowerNextError::cancelled()))
        })
    }
}

/// The error type [`TowerNext`] returns.
///
/// Middleware should let this error pass through unchanged: the enclosing
/// [`TowerLayer`] restores an error produced by the wrapped routes to the
/// original [`Error`] value. The other cases are misuse (calling the service
/// a second time, or from a request that lost the original request's
/// extensions) and the request being cancelled.
#[derive(Debug)]
pub struct TowerNextError {
    repr: Repr,
}

/// The cases a [`TowerNextError`] distinguishes.
#[derive(Debug)]
enum Repr {
    /// The wrapped chain produced this error; the adapter unwraps it.
    Tunneled(Error),
    /// The chain was called a second time.
    Consumed,
    /// The request no longer carries the relay to its adapter.
    Detached,
    /// The adapter was dropped before the chain produced a response.
    Cancelled,
}

impl TowerNextError {
    /// Wraps an error produced by the wrapped chain for the trip across the
    /// tower stack.
    fn tunneled(error: Error) -> Self {
        Self {
            repr: Repr::Tunneled(error),
        }
    }

    /// The chain was called a second time.
    fn consumed() -> Self {
        Self {
            repr: Repr::Consumed,
        }
    }

    /// The request no longer carries the relay to its adapter.
    fn detached() -> Self {
        Self {
            repr: Repr::Detached,
        }
    }

    /// The adapter was dropped before the chain produced a response.
    fn cancelled() -> Self {
        Self {
            repr: Repr::Cancelled,
        }
    }
}

impl Display for TowerNextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.repr {
            Repr::Tunneled(error) => Display::fmt(error, f),
            Repr::Consumed => f.write_str("the chain wrapped by this TowerLayer has already run"),
            Repr::Detached => {
                f.write_str("the request no longer carries the relay to its TowerLayer")
            }
            Repr::Cancelled => {
                f.write_str("the TowerLayer was dropped before the chain produced a response")
            }
        }
    }
}

impl std::error::Error for TowerNextError {}

/// An error a tower service produced itself, as opposed to one that passed
/// through it from wrapped routes.
///
/// Both adapters surface it: a [`TowerLayer`] wraps a failure of its
/// middleware (a timeout elapsing, a load-shed rejection), and a
/// [`TowerRoute`] wraps an error returned by its mounted service. An outer
/// layer can downcast to it to map specific failures onto responses;
/// unmapped, the router renders it as a 500.
#[derive(Debug)]
pub struct TowerServiceError(BoxError);

impl TowerServiceError {
    /// Returns a reference to the underlying error.
    #[must_use]
    pub fn get_ref(&self) -> &BoxError {
        &self.0
    }

    /// Consumes the wrapper, returning the underlying error.
    #[must_use]
    pub fn into_inner(self) -> BoxError {
        self.0
    }
}

impl Display for TowerServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("tower service error")
    }
}

impl std::error::Error for TowerServiceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

/// A call from [`TowerNext`] back into the wrapped chain: the (possibly
/// modified) request, and the sender the chain's result is returned on.
type ChainCall = (Request, oneshot::Sender<Result<Response, TowerNextError>>);

/// The sending half of the channel over which [`TowerNext`] reaches back into
/// the adapter that spawned it, carried across the middleware in the request
/// extensions.
#[derive(Clone)]
struct Relay(mpsc::Sender<ChainCall>);

/// Creates the per-request relay channel between [`TowerNext`] and the
/// adapter driving the request.
fn relay_channel() -> (Relay, mpsc::Receiver<ChainCall>) {
    // Capacity 1 suffices: the chain runs at most once, and repeated calls are
    // answered with an error as they arrive.
    let (sender, receiver) = mpsc::channel(1);
    (Relay(sender), receiver)
}

/// Converts the middleware's outcome into the chain's result, mapping the
/// response body back to [`Body`] and recovering tunneled errors.
fn finish<ResBody, E>(result: Result<http::Response<ResBody>, E>) -> Result<Response>
where
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
    E: Into<BoxError>,
{
    match result {
        Ok(response) => Ok(response.map(Body::new)),
        Err(error) => Err(recover(error.into())),
    }
}

/// Maps an error surfacing from a tower stack back onto a topcoat [`Error`]:
/// an error tunneled from the wrapped chain is unwrapped to its original
/// value, while an error the middleware produced itself is wrapped in a
/// [`TowerServiceError`].
fn recover(error: BoxError) -> Error {
    match error.downcast::<TowerNextError>() {
        Ok(error) => match error.repr {
            Repr::Tunneled(error) => error,
            repr => TowerNextError { repr }.into(),
        },
        Err(error) => TowerServiceError(error).into(),
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::convert::Infallible;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use http::request::Parts;
    use http::{HeaderValue, StatusCode};
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{
        Bytes, IntoResponse, Layers, Method, RouteFn, RouteFuture, Router, Terminal,
        error::NotFoundError, to_bytes,
    };

    // -- Test helpers --

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn path(s: &'static str) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new(s))
    }

    /// Builds a request context carrying the parts of a GET request to `uri`.
    fn cx_for(uri: &str) -> CxBuilder {
        let (parts, ()) = http::Request::builder()
            .uri(uri)
            .body(())
            .unwrap()
            .into_parts();
        let mut cx = CxBuilder::default();
        cx.insert(parts);
        cx
    }

    /// Runs a request through `layer` wrapped directly around `route`.
    fn run(layer: &dyn Layer, cx: &mut CxBuilder, route: &RouteFn) -> Result<Response> {
        let layers = Layers::default();
        let next = Next::new(&layers, &[], Terminal::Route(route));
        block_on(layer.handle(cx, Body::empty(), next))
    }

    /// Reads a response body to completion.
    fn body_bytes(response: Response) -> Bytes {
        let (_, body) = response.into_parts();
        block_on(to_bytes(body, usize::MAX)).unwrap()
    }

    fn say_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "route".into_response(cx) })
    }

    /// Echoes the `x-tower` request header, so a test can observe request
    /// edits made by middleware.
    fn echo_header(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            let value = crate::headers(cx)
                .get("x-tower")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("missing")
                .to_owned();
            value.into_response(cx)
        })
    }

    /// A route that never resolves, for racing against a timeout middleware.
    fn hang(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(std::future::pending())
    }

    /// A route whose body is long enough to clear tower-http's compression
    /// size threshold.
    fn long_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "route ".repeat(64).into_response(cx) })
    }

    /// A mountable service echoing the request's method, URI, and body, to
    /// observe exactly what crosses a [`TowerRoute`].
    async fn echo_service(request: Request) -> Result<Response, Infallible> {
        let (parts, body) = request.into_parts();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        let reply = format!(
            "{} {} {}",
            parts.method,
            parts.uri,
            String::from_utf8_lossy(&bytes)
        );
        Ok(Response::new(Body::from(reply)))
    }

    /// Dispatches a GET request for `uri` through a full router.
    fn send(router: &Router, uri: &str) -> Response {
        let request = http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        block_on(router.handle(request))
    }

    /// A middleware that stamps an `x-tower` header onto the request.
    struct MarkRequestLayer;

    impl<S> tower::Layer<S> for MarkRequestLayer {
        type Service = MarkRequest<S>;

        fn layer(&self, inner: S) -> Self::Service {
            MarkRequest { inner }
        }
    }

    #[derive(Clone)]
    struct MarkRequest<S> {
        inner: S,
    }

    impl<S> tower::Service<Request> for MarkRequest<S>
    where
        S: tower::Service<Request>,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, mut request: Request) -> Self::Future {
            request
                .headers_mut()
                .insert("x-tower", HeaderValue::from_static("marked"));
            self.inner.call(request)
        }
    }

    /// A middleware that stamps an `x-tower` header onto the response.
    struct MarkResponseLayer;

    impl<S> tower::Layer<S> for MarkResponseLayer {
        type Service = MarkResponse<S>;

        fn layer(&self, inner: S) -> Self::Service {
            MarkResponse { inner }
        }
    }

    #[derive(Clone)]
    struct MarkResponse<S> {
        inner: S,
    }

    impl<S> tower::Service<Request> for MarkResponse<S>
    where
        S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
        S::Future: Send,
    {
        type Response = Response;
        type Error = S::Error;
        type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let mut inner = self.inner.clone();
            Box::pin(async move {
                let mut response = inner.call(request).await?;
                response
                    .headers_mut()
                    .insert("x-tower", HeaderValue::from_static("marked"));
                Ok(response)
            })
        }
    }

    /// A middleware that answers the request itself, never calling the chain.
    struct ShortCircuitLayer;

    impl<S> tower::Layer<S> for ShortCircuitLayer {
        type Service = ShortCircuit;

        fn layer(&self, _inner: S) -> Self::Service {
            ShortCircuit
        }
    }

    #[derive(Clone)]
    struct ShortCircuit;

    impl tower::Service<Request> for ShortCircuit {
        type Response = Response;
        type Error = Infallible;
        type Future = std::future::Ready<Result<Response, Infallible>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, request: Request) -> Self::Future {
            drop(request);
            std::future::ready(Ok(Response::new(Body::from("short"))))
        }
    }

    /// A layer that counts how often it builds its service and how many
    /// requests the built service handles, to pin the build-once contract.
    struct CountingLayer {
        builds: Arc<AtomicUsize>,
        requests: Arc<AtomicUsize>,
    }

    impl<S> tower::Layer<S> for CountingLayer {
        type Service = Counting<S>;

        fn layer(&self, inner: S) -> Self::Service {
            self.builds.fetch_add(1, Ordering::SeqCst);
            Counting {
                requests: self.requests.clone(),
                inner,
            }
        }
    }

    #[derive(Clone)]
    struct Counting<S> {
        requests: Arc<AtomicUsize>,
        inner: S,
    }

    impl<S> tower::Service<Request> for Counting<S>
    where
        S: tower::Service<Request>,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, request: Request) -> Self::Future {
            self.requests.fetch_add(1, Ordering::SeqCst);
            self.inner.call(request)
        }
    }

    /// A middleware that calls the chain twice, the way a retry would.
    struct CallTwiceLayer;

    impl<S> tower::Layer<S> for CallTwiceLayer {
        type Service = CallTwice<S>;

        fn layer(&self, inner: S) -> Self::Service {
            CallTwice { inner }
        }
    }

    #[derive(Clone)]
    struct CallTwice<S> {
        inner: S,
    }

    impl<S> tower::Service<Request> for CallTwice<S>
    where
        S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
        S::Future: Send,
    {
        type Response = Response;
        type Error = S::Error;
        type Future = Pin<Box<dyn Future<Output = Result<Response, S::Error>> + Send>>;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, request: Request) -> Self::Future {
            let mut inner = self.inner.clone();
            Box::pin(async move {
                // Keep a copy of the extensions (with the adapter's relay), the
                // way a retrying middleware would clone the request up front.
                let extensions = request.extensions().clone();
                inner.call(request).await?;
                let mut retry = Request::new(Body::empty());
                *retry.extensions_mut() = extensions;
                inner.call(retry).await
            })
        }
    }

    /// A middleware that swaps in a fresh request, losing the adapter's relay.
    struct DetachLayer;

    impl<S> tower::Layer<S> for DetachLayer {
        type Service = Detach<S>;

        fn layer(&self, inner: S) -> Self::Service {
            Detach { inner }
        }
    }

    #[derive(Clone)]
    struct Detach<S> {
        inner: S,
    }

    impl<S> tower::Service<Request> for Detach<S>
    where
        S: tower::Service<Request>,
    {
        type Response = S::Response;
        type Error = S::Error;
        type Future = S::Future;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, request: Request) -> Self::Future {
            drop(request);
            self.inner.call(Request::new(Body::empty()))
        }
    }

    // -- TowerLayer --

    #[test]
    fn tower_layer_exposes_its_path() {
        let layer = TowerLayer::new(Path::new("/admin"), tower::layer::util::Identity::new());
        assert_eq!(layer.path(), Path::new("/admin"));
    }

    #[test]
    fn passes_the_request_through_to_the_route() {
        let layer = TowerLayer::new(Path::new("/"), tower::layer::util::Identity::new());
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let mut cx = cx_for("/x");

        let response = run(&layer, &mut cx, &route).unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"route");
    }

    #[test]
    fn request_edits_reach_the_route_and_the_context() {
        let layer = TowerLayer::new(Path::new("/"), MarkRequestLayer);
        let route = RouteFn::new(Method::GET, path("/x"), echo_header);
        let mut cx = cx_for("/x");

        let response = run(&layer, &mut cx, &route).unwrap();

        // The route saw the header the middleware added, and the modified
        // request was written back to the context.
        assert_eq!(&body_bytes(response)[..], b"marked");
        assert!(cx.get::<Parts>().unwrap().headers.contains_key("x-tower"));
    }

    #[test]
    fn response_edits_reach_the_caller() {
        let layer = TowerLayer::new(Path::new("/"), MarkResponseLayer);
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let mut cx = cx_for("/x");

        let response = run(&layer, &mut cx, &route).unwrap();

        assert_eq!(response.headers().get("x-tower").unwrap(), "marked");
        assert_eq!(&body_bytes(response)[..], b"route");
    }

    #[test]
    fn middleware_can_short_circuit_without_calling_the_chain() {
        let layer = TowerLayer::new(Path::new("/"), ShortCircuitLayer);
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let mut cx = cx_for("/x");

        let response = run(&layer, &mut cx, &route).unwrap();

        assert_eq!(&body_bytes(response)[..], b"short");
        // The chain never ran, so the parts stay on the context for outer
        // layers and error rendering.
        assert!(cx.get::<Parts>().is_some());
    }

    #[test]
    fn chain_errors_tunnel_through_unchanged() {
        let layer = TowerLayer::new(Path::new("/"), tower::layer::util::Identity::new());
        let layers = Layers::default();
        let mut cx = cx_for("/missing");

        let next = Next::new(&layers, &[], Terminal::NotFound);
        let result = block_on(layer.handle(&mut cx, Body::empty(), next));

        // The 404 comes back out as the original typed error, not a response.
        assert!(
            result
                .unwrap_err()
                .downcast_ref::<NotFoundError>()
                .is_some()
        );
    }

    #[test]
    fn chain_errors_tunnel_through_an_error_boxing_middleware() {
        // `Timeout` boxes its inner service's errors; the original error must
        // still be recovered on the way out.
        let layer = TowerLayer::new(
            Path::new("/"),
            tower::timeout::TimeoutLayer::new(Duration::from_mins(1)),
        );
        let layers = Layers::default();
        let mut cx = cx_for("/missing");

        let next = Next::new(&layers, &[], Terminal::NotFound);
        let result = block_on(layer.handle(&mut cx, Body::empty(), next));

        assert!(
            result
                .unwrap_err()
                .downcast_ref::<NotFoundError>()
                .is_some()
        );
    }

    #[test]
    fn middleware_is_built_once_and_shared_across_requests() {
        let builds = Arc::new(AtomicUsize::new(0));
        let requests = Arc::new(AtomicUsize::new(0));
        let layer = TowerLayer::new(
            Path::new("/"),
            CountingLayer {
                builds: builds.clone(),
                requests: requests.clone(),
            },
        );
        assert_eq!(builds.load(Ordering::SeqCst), 1);

        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        for _ in 0..2 {
            let mut cx = cx_for("/x");
            run(&layer, &mut cx, &route).unwrap();
        }

        // The tower layer built one service; its per-request clones shared it.
        assert_eq!(builds.load(Ordering::SeqCst), 1);
        assert_eq!(requests.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn timeout_middleware_cancels_a_hung_route() {
        let layer = TowerLayer::new(
            Path::new("/"),
            tower::timeout::TimeoutLayer::new(Duration::from_millis(10)),
        );
        let route = RouteFn::new(Method::GET, path("/x"), hang);
        let mut cx = cx_for("/x");

        // The route never resolves; the timeout must fire while the chain is
        // in flight, which requires the middleware to stay polled.
        let error = run(&layer, &mut cx, &route).unwrap_err();

        let middleware = error.downcast_ref::<TowerServiceError>().unwrap();
        assert!(middleware.get_ref().is::<tower::timeout::error::Elapsed>());
    }

    #[test]
    fn calling_the_chain_twice_errors() {
        let layer = TowerLayer::new(Path::new("/"), CallTwiceLayer);
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let mut cx = cx_for("/x");

        let error = run(&layer, &mut cx, &route).unwrap_err();
        assert!(error.downcast_ref::<TowerNextError>().is_some());
    }

    #[test]
    fn calling_the_chain_without_the_relay_errors() {
        let layer = TowerLayer::new(Path::new("/"), DetachLayer);
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let mut cx = cx_for("/x");

        let error = run(&layer, &mut cx, &route).unwrap_err();
        assert!(error.downcast_ref::<TowerNextError>().is_some());
    }

    // -- Ecosystem middleware, registered through the router --

    #[test]
    fn works_with_tower_concurrency_limit() {
        let router = Router::builder()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .layer(TowerLayer::new(
                Path::new("/"),
                tower::limit::ConcurrencyLimitLayer::new(1),
            ))
            .build();

        // The permit taken for the first request is released for the second.
        for _ in 0..2 {
            let response = send(&router, "/x");
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(&body_bytes(response)[..], b"route");
        }
    }

    #[test]
    fn works_with_tower_buffer_and_rate_limit() {
        // `RateLimit` is not `Clone`; the documented pattern wraps it in
        // `tower::buffer`, whose handle is. `Buffer` spawns its worker task, so
        // the adapter must be built inside a runtime.
        block_on(async {
            let router = Router::builder()
                .route(RouteFn::new(Method::GET, path("/x"), say_route))
                .layer(TowerLayer::new(
                    Path::new("/"),
                    tower::ServiceBuilder::new()
                        .buffer::<Request>(8)
                        .rate_limit(100, Duration::from_secs(1))
                        .into_inner(),
                ))
                .build();

            for _ in 0..2 {
                let request = http::Request::builder()
                    .uri("/x")
                    .body(Body::empty())
                    .unwrap();
                let response = router.handle(request).await;
                assert_eq!(response.status(), StatusCode::OK);
            }
        });
    }

    #[test]
    fn works_with_tower_http_set_response_header() {
        let router = Router::builder()
            .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
            .route(RouteFn::new(Method::GET, path("/public"), say_route))
            .layer(TowerLayer::new(
                Path::new("/admin"),
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    http::header::HeaderName::from_static("x-tower"),
                    HeaderValue::from_static("marked"),
                ),
            ))
            .build();

        let response = send(&router, "/admin/x");
        assert_eq!(response.headers().get("x-tower").unwrap(), "marked");

        // The middleware only wraps routes under its path.
        let response = send(&router, "/public");
        assert!(!response.headers().contains_key("x-tower"));
    }

    #[test]
    fn works_with_tower_http_cors() {
        let router = Router::builder()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .layer(TowerLayer::new(
                Path::new("/"),
                tower_http::cors::CorsLayer::permissive(),
            ))
            .build();

        // The middleware answers a preflight request itself; without it the
        // router would return a 405 for OPTIONS.
        let request = http::Request::builder()
            .method(Method::OPTIONS)
            .uri("/x")
            .header(http::header::ORIGIN, "https://example.com")
            .header(http::header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
            .body(Body::empty())
            .unwrap();
        let response = block_on(router.handle(request));
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .contains_key(http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
        );

        // A plain request flows through to the route, with CORS headers added.
        let response = send(&router, "/x");
        assert!(
            response
                .headers()
                .contains_key(http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
        );
        assert_eq!(&body_bytes(response)[..], b"route");
    }

    #[test]
    fn works_with_tower_http_compression() {
        let router = Router::builder()
            .route(RouteFn::new(Method::GET, path("/x"), long_route))
            .layer(TowerLayer::new(
                Path::new("/"),
                tower_http::compression::CompressionLayer::new(),
            ))
            .build();

        let request = http::Request::builder()
            .uri("/x")
            .header(http::header::ACCEPT_ENCODING, "gzip")
            .body(Body::empty())
            .unwrap();
        let response = block_on(router.handle(request));

        // The middleware's wrapped body type crossed back through the adapter.
        assert_eq!(
            response
                .headers()
                .get(http::header::CONTENT_ENCODING)
                .unwrap(),
            "gzip"
        );
        let compressed = body_bytes(response);
        assert!(!compressed.is_empty());
        assert!(compressed.len() < "route ".repeat(64).len());
    }

    #[test]
    fn works_with_tower_http_trace() {
        let router = Router::builder()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .layer(TowerLayer::new(
                Path::new("/"),
                tower_http::trace::TraceLayer::new_for_http(),
            ))
            .build();

        let response = send(&router, "/x");
        assert_eq!(response.status(), StatusCode::OK);

        // A tunneled 404 satisfies the classifier's error bounds and still
        // renders at the router's edge.
        let response = send(&router, "/missing");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn works_with_tower_http_timeout() {
        let router = Router::builder()
            .route(RouteFn::new(Method::GET, path("/x"), hang))
            .layer(TowerLayer::new(
                Path::new("/"),
                tower_http::timeout::TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_millis(10),
                ),
            ))
            .build();

        // Unlike tower's timeout, tower-http's renders a 408 response.
        let response = send(&router, "/x");
        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    }

    // -- TowerRoute --

    #[test]
    fn tower_route_exposes_its_methods_and_path() {
        let route = TowerRoute::new(
            Method::POST,
            Path::new("/legacy"),
            tower::service_fn(echo_service),
        );
        assert_eq!(route.methods(), Methods::Only(&[Method::POST]));
        assert_eq!(route.path(), Path::new("/legacy"));

        let route = TowerRoute::new(
            Methods::Any,
            Path::new("/legacy"),
            tower::service_fn(echo_service),
        );
        assert_eq!(route.methods(), Methods::Any);
    }

    #[test]
    fn mounts_a_service_at_a_catch_all_path() {
        let router = Router::builder()
            .route(TowerRoute::new(
                Methods::Any,
                Path::new("/legacy/{*rest}"),
                tower::service_fn(echo_service),
            ))
            .build();

        // The service sees the original method, URI, and body: nothing is
        // stripped or rewritten on the way in.
        let request = http::Request::builder()
            .method(Method::POST)
            .uri("/legacy/users/7?page=2")
            .body(Body::from("payload"))
            .unwrap();
        let response = block_on(router.handle(request));

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            &body_bytes(response)[..],
            b"POST /legacy/users/7?page=2 payload"
        );
    }

    #[test]
    fn a_tower_route_serves_only_its_declared_methods() {
        let router = Router::builder()
            .route(TowerRoute::new(
                Method::POST,
                Path::new("/legacy"),
                tower::service_fn(echo_service),
            ))
            .build();

        let request = http::Request::builder()
            .method(Method::POST)
            .uri("/legacy")
            .body(Body::empty())
            .unwrap();
        assert_eq!(block_on(router.handle(request)).status(), StatusCode::OK);

        let response = send(&router, "/legacy");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn layers_wrap_a_mounted_service() {
        let router = Router::builder()
            .route(TowerRoute::new(
                Methods::Any,
                Path::new("/legacy/{*rest}"),
                tower::service_fn(echo_service),
            ))
            .layer(TowerLayer::new(
                Path::new("/legacy"),
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    http::header::HeaderName::from_static("x-tower"),
                    HeaderValue::from_static("marked"),
                ),
            ))
            .build();

        let response = send(&router, "/legacy/x");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-tower").unwrap(), "marked");
    }

    #[test]
    fn a_mounted_service_error_surfaces_as_a_tower_service_error() {
        let failing = tower::service_fn(|_request: Request| async {
            Err::<Response, _>(std::io::Error::other("legacy failure"))
        });
        let route = TowerRoute::new(Methods::Any, Path::new("/legacy"), failing);
        let cx = cx_for("/legacy");

        let error = block_on(route.handle(&cx, Body::empty())).unwrap_err();

        let route_error = error.downcast_ref::<TowerServiceError>().unwrap();
        assert!(route_error.get_ref().is::<std::io::Error>());
    }
}
