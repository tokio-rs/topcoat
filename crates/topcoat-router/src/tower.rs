use std::fmt::{self, Display};
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::{Mutex, PoisonError};
use std::task::{Context, Poll};

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};
use topcoat_core::context::CxBuilder;
use topcoat_core::error::{Error, Result};
use tower::ServiceExt;

use crate::{Body, BoxError, Layer, LayerFuture, Next, Path, Request, Response};

/// A [`Layer`] that wraps the routes nested under its path in a
/// [`tower::Layer`]'s middleware.
///
/// The tower layer is applied once, at construction, to a [`TowerNext`]
/// service standing in for the rest of the chain, so middleware state (a
/// concurrency-limit semaphore, a rate-limit window) is shared across requests
/// just like in a plain tower stack. Each request then runs on a clone of the
/// composed service.
///
/// The middleware sees the request as an [`http::Request`] carrying [`Body`],
/// reassembled from the parts stored on the request context. Modifications the
/// middleware makes to the request (added headers, a rewritten URI, new
/// extensions) are written back to the context before the wrapped chain runs,
/// so inner layers and the route observe them.
///
/// Errors keep their topcoat semantics: an `Err` produced by the wrapped chain
/// (a 404, a handler error) tunnels through the tower stack inside a
/// [`TowerNextError`] and is unwrapped on the way out, so layers outside the
/// adapter observe the original [`Error`] value. An error produced by the
/// middleware itself (a timeout elapsing, a load-shed rejection) surfaces as
/// an `Err` wrapping a [`MiddlewareError`].
///
/// To run several tower layers, compose them first (for example with
/// [`tower::ServiceBuilder`], whose `into_inner` returns the composed layer
/// stack) and wrap the result in a single `TowerLayer`. Middleware that spawns
/// its inner service (like `tower::buffer`) works, since [`TowerNext`] and its
/// futures are `'static`. Middleware that calls its inner service more than
/// once per request (like `tower::retry`) does not: the wrapped chain runs at
/// most once, and a repeated call resolves to an error.
///
/// Register the adapter with
/// [`RouterBuilder::layer`](crate::RouterBuilder::layer).
///
/// # Examples
///
/// ```rust
/// use std::time::Duration;
///
/// use topcoat::router::{Path, Router, TowerLayer};
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
    path: &'static Path,
    /// The composed tower service, built once and cloned per request. The
    /// mutex only guards the clone; it is never held across a call.
    service: Mutex<S>,
}

impl<S> TowerLayer<S> {
    /// Wraps the routes under `path` in the middleware `layer` builds.
    ///
    /// Applies `layer` to a [`TowerNext`] immediately; the resulting service
    /// handles every request passing through this adapter.
    #[must_use]
    pub fn new<L>(path: &'static Path, layer: L) -> Self
    where
        L: tower::Layer<TowerNext, Service = S>,
    {
        Self {
            path,
            service: Mutex::new(layer.layer(TowerNext::new())),
        }
    }
}

impl<S, ResBody> Layer for TowerLayer<S>
where
    S: tower::Service<Request, Response = http::Response<ResBody>> + Clone + Send + 'static,
    S::Error: Into<BoxError>,
    S::Future: Send,
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<BoxError>,
{
    fn path(&self) -> &Path {
        self.path
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        // Clones of a tower service share its cross-request state (semaphores,
        // rate-limit windows) through the service's internal handles. Poison is
        // harmless here since the lock only ever guards this clone.
        let service = self
            .service
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        Box::pin(async move {
            // Reassemble the http request the middleware operates on from the
            // parts stored on the context, and slip it the relay over which
            // `TowerNext` calls back into this chain.
            let mut request = Request::from_parts(crate::parts(cx).clone(), body);
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

/// The tower service standing in for the wrapped chain inside a
/// [`TowerLayer`]'s middleware stack.
///
/// [`TowerLayer::new`] hands this service to the given [`tower::Layer`], so
/// the middleware it builds wraps the inner topcoat layers and the route.
/// Calling the service forwards the request to that chain and resolves with
/// the chain's response. The chain behind a request runs at most once: a
/// middleware that calls the service again (like a retry) receives a
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
/// An `Err` produced by the wrapped chain crosses the tower stack inside this
/// type, and the enclosing [`TowerLayer`] unwraps it on the way out, so layers
/// outside the adapter observe the original [`Error`]. The remaining cases,
/// which surface as-is, are misuse (calling the chain twice, or from a request
/// that no longer carries the adapter's relay) and cancellation.
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

/// An error a [`TowerLayer`]'s middleware produced itself, as opposed to one
/// tunneled through it from the wrapped chain.
///
/// The adapter wraps middleware failures (a timeout elapsing, a load-shed
/// rejection) in this type before surfacing them as an [`Error`]. An outer
/// layer can downcast to it to map specific middleware failures onto
/// responses; unmapped, the router renders it as a 500.
#[derive(Debug)]
pub struct MiddlewareError(BoxError);

impl MiddlewareError {
    /// Returns a reference to the middleware's underlying error.
    #[must_use]
    pub fn get_ref(&self) -> &BoxError {
        &self.0
    }

    /// Consumes the wrapper, returning the middleware's underlying error.
    #[must_use]
    pub fn into_inner(self) -> BoxError {
        self.0
    }
}

impl Display for MiddlewareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("tower middleware error")
    }
}

impl std::error::Error for MiddlewareError {
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
/// [`MiddlewareError`].
fn recover(error: BoxError) -> Error {
    match error.downcast::<TowerNextError>() {
        Ok(error) => match error.repr {
            Repr::Tunneled(error) => error,
            repr => TowerNextError { repr }.into(),
        },
        Err(error) => MiddlewareError(error).into(),
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
        Bytes, IntoResponse, Layers, Method, NotFoundError, RouteFn, RouteFuture, Terminal,
        to_bytes,
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
        assert!(result.unwrap_err().downcast_ref::<NotFoundError>().is_some());
    }

    #[test]
    fn chain_errors_tunnel_through_an_error_boxing_middleware() {
        // `Timeout` boxes its inner service's errors; the original error must
        // still be recovered on the way out.
        let layer = TowerLayer::new(
            Path::new("/"),
            tower::timeout::TimeoutLayer::new(Duration::from_secs(60)),
        );
        let layers = Layers::default();
        let mut cx = cx_for("/missing");

        let next = Next::new(&layers, &[], Terminal::NotFound);
        let result = block_on(layer.handle(&mut cx, Body::empty(), next));

        assert!(result.unwrap_err().downcast_ref::<NotFoundError>().is_some());
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

        let middleware = error.downcast_ref::<MiddlewareError>().unwrap();
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
}
