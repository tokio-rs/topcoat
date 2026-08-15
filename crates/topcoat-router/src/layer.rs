use std::{borrow::Cow, pin::Pin, sync::Arc};

use topcoat_core::{context::Cx, error::Result};

use crate::{Body, Endpoint, IntoPath, Path, Route, error::method_not_allowed, response::Response};

/// The future returned by [`Layer::handle`] and [`Next::run`]: a boxed, `Send`
/// future borrowing the chain and the request context.
pub type LayerFuture<'a> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>;

/// A request-processing layer that wraps the routes nested under its path,
/// similar to a tower middleware.
///
/// A layer wraps every matched route whose path begins with the layer's path
/// (the same prefix rule as layouts), so a layer at `/admin` wraps only routes
/// under `/admin`, while a layer at `/` wraps everything. Each layer receives
/// the [`Cx`] and the request [`Body`], plus a [`Next`] representing the rest
/// of the chain. A layer typically derives a child context carrying
/// request-scoped values with [`Cx::with`], passes it to [`Next::run`] to
/// invoke the inner layers and ultimately the route, then inspects or modifies
/// the [`Response`].
///
/// When several layers match a route they nest from least-specific (outermost)
/// to most-specific (innermost), like layouts.
///
/// Register layers with [`RouterBuilder::layer`](crate::RouterBuilder::layer).
///
/// # Examples
///
/// ```rust
/// use std::borrow::Cow;
///
/// use topcoat::{
///     context::Cx,
///     router::{Body, Layer, LayerFuture, Next, Path},
/// };
///
/// struct Timing;
///
/// impl Layer for Timing {
///     fn path(&self) -> &Path {
///         Path::ROOT
///     }
///
///     fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
///         Box::pin(async move {
///             let start = std::time::Instant::now();
///             let response = next.run(cx, body).await?;
///             println!("handled in {:?}", start.elapsed());
///             Ok(response)
///         })
///     }
/// }
/// ```
pub trait Layer: Send + Sync + 'static {
    /// The URL path prefix whose routes this layer wraps.
    fn path(&self) -> &Path;

    /// Handles a request, calling `next` to continue down the chain.
    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a>;
}

impl<L: Layer + ?Sized> Layer for &'static L {
    fn path(&self) -> &Path {
        (**self).path()
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        (**self).handle(cx, body, next)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Layer);

/// The handler function backing a [`LayerFn`].
pub type LayerHandlerFn = for<'a> fn(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a>;

/// A [`Layer`] backed by a plain handler function.
///
/// Turns a function into a layer without implementing [`Layer`] on a struct,
/// pairing it with the path prefix it applies to.
#[derive(Debug, Clone)]
pub struct LayerFn {
    /// The URL path prefix whose routes this layer wraps.
    path: Cow<'static, Path>,
    /// The handler function that wraps the inner chain.
    handle: LayerHandlerFn,
}

impl LayerFn {
    /// Creates a new layer with an explicit path prefix and handler function.
    ///
    /// # Panics
    ///
    /// Panics if `path` is a string that is not a well-formed route path.
    #[track_caller]
    pub fn new(path: impl IntoPath, handle: LayerHandlerFn) -> Self {
        Self {
            path: path.into_path(),
            handle,
        }
    }
}

impl Layer for LayerFn {
    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        (self.handle)(cx, body, next)
    }
}

/// Selects the layers in `layers` whose path is a prefix of `path`, ordered
/// least- to most-specific so the outermost layer runs first. Among layers
/// that share a path, the later one in `layers` runs first.
pub(crate) fn layers_for_path(layers: &[Arc<dyn Layer>], path: &Path) -> Box<[Arc<dyn Layer>]> {
    let mut matching: Vec<&Arc<dyn Layer>> = layers
        .iter()
        .filter(|layer| path.starts_with(layer.path()))
        .rev()
        .collect();
    matching.sort_by_key(|layer| layer.path().len());
    matching.into_iter().cloned().collect()
}

/// What a [`Next`] chain runs once its layers are exhausted.
///
/// A matched route runs inside the layer stack selected for its own path,
/// group segments included. When the path matches but the method does not,
/// the 405 runs inside the stack selected for the endpoint's URL path, so a
/// layer sees a matched route handler's result, or the method-not-allowed
/// error, uniformly as the `Result` returned by [`Next::run`].
#[derive(Clone, Copy)]
pub(crate) enum Terminal<'a> {
    /// A matched route handles the request.
    Route(&'a dyn Route),
    /// The path matched but the method did not; the chain resolves to a
    /// method-not-allowed error listing the endpoint's supported methods.
    MethodNotAllowed(&'a Endpoint),
}

/// The continuation of a [`Layer`] chain: the remaining layers followed by the
/// chain's terminal handler.
///
/// Passed as the `next` argument to [`Layer::handle`]. Call [`run`](Self::run)
/// to invoke the next layer, or the terminal once the layers are exhausted.
pub struct Next<'a> {
    /// The layers wrapping this request, ordered from least- to most-specific
    /// so the outermost layer runs first.
    layers: &'a [Arc<dyn Layer>],
    /// What runs once the layers are exhausted.
    terminal: Terminal<'a>,
}

impl<'a> Next<'a> {
    /// Creates a chain that runs `layers` (in order), then `terminal`.
    ///
    /// `layers` must be ordered from least- to most-specific (ascending path
    /// length), so the outermost layer runs first.
    pub(crate) fn new(layers: &'a [Arc<dyn Layer>], terminal: Terminal<'a>) -> Self {
        Self { layers, terminal }
    }

    /// Runs the next layer in the chain, or the terminal handler once no layers
    /// remain.
    #[must_use]
    pub fn run(self, cx: &'a Cx, body: Body) -> LayerFuture<'a> {
        match self.layers.split_first() {
            Some((layer, rest)) => layer.handle(
                cx,
                body,
                Next {
                    layers: rest,
                    ..self
                },
            ),
            None => match self.terminal {
                Terminal::Route(route) => route.handle(cx, body),
                Terminal::MethodNotAllowed(endpoint) => {
                    let error = method_not_allowed(endpoint.methods().cloned());
                    Box::pin(async move { Err(error.into()) })
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future::Future,
        sync::{Arc, Mutex},
    };

    use http::StatusCode;
    use topcoat_core::context::{AppContext, Cx, app_context};

    use super::*;
    use crate::{
        Method, RouteFn, RouteFuture, RouteIndex, error::respond, request::Bytes,
        response::IntoResponse, to_bytes,
    };

    // -- Test helpers --

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn path(s: &'static str) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new(s))
    }

    /// A layer whose path is all a test cares about; its handler just forwards
    /// to the rest of the chain and never runs in the selection tests.
    fn layer_at(p: &'static str) -> Arc<dyn Layer> {
        Arc::new(LayerFn::new(path(p), noop_layer))
    }

    /// Asserts that `layers_for_path` selects the layers at the `expected`
    /// paths, in order.
    fn assert_selects(layers: &[Arc<dyn Layer>], p: &'static str, expected: &[&'static str]) {
        let selected = layers_for_path(layers, Path::new(p));
        let paths: Vec<&Path> = selected.iter().map(|layer| layer.path()).collect();
        let expected: Vec<&Path> = expected.iter().map(|e| Path::new(e)).collect();
        assert_eq!(paths, expected);
    }

    fn noop_layer<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        next.run(cx, body)
    }

    /// Reads a response body to completion.
    fn body_bytes(response: Response) -> Bytes {
        let (_, body) = response.into_parts();
        block_on(to_bytes(body, usize::MAX)).unwrap()
    }

    /// A shared log of the labels layers and routes record as they run, so a
    /// test can observe the order the chain executes in.
    type Trace = Mutex<Vec<&'static str>>;

    fn cx_with_trace(trace: Arc<Trace>) -> Cx {
        let mut app = AppContext::new();
        app.insert(trace);
        Cx::new(Arc::new(app))
    }

    fn record_a<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("a");
            next.run(cx, body).await
        })
    }

    fn record_b<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("b");
            next.run(cx, body).await
        })
    }

    /// A layer that answers the request itself, without invoking `next`.
    fn short_circuit<'a>(cx: &'a Cx, _body: Body, _next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move { "short".into_response(cx) })
    }

    fn say_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "route".into_response(cx) })
    }

    fn record_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("route");
            "route".into_response(cx)
        })
    }

    // -- LayerFn --

    #[test]
    fn layer_fn_exposes_its_path() {
        let layer = LayerFn::new(path("/admin"), noop_layer);
        assert_eq!(layer.path(), Path::new("/admin"));
    }

    // -- layers_for_path --

    #[test]
    fn for_path_orders_prefix_layers_least_to_most_specific() {
        let layers = [layer_at("/"), layer_at("/users"), layer_at("/posts")];
        // The route at /users/{id} is wrapped by the root and /users layers, in
        // that order; the /posts layer does not prefix it.
        assert_selects(&layers, "/users/{id}", &["/", "/users"]);
    }

    #[test]
    fn for_path_runs_the_later_of_a_shared_path_first() {
        let first = layer_at("/admin");
        let second = layer_at("/admin");
        let layers = [Arc::clone(&first), Arc::clone(&second)];
        let selected = layers_for_path(&layers, Path::new("/admin/users"));
        assert_eq!(selected.len(), 2);
        assert!(Arc::ptr_eq(&selected[0], &second));
        assert!(Arc::ptr_eq(&selected[1], &first));
    }

    #[test]
    fn for_path_rejects_partial_segments() {
        let layers = [layer_at("/admin")];
        assert!(layers_for_path(&layers, Path::new("/administrator")).is_empty());
    }

    #[test]
    fn for_path_includes_group_segments() {
        let layers = [layer_at("/(auth)"), layer_at("/dashboard")];
        // Groups are part of the logical path: the layer inside `(auth)` wraps
        // the endpoint, while the URL-lookalike `/dashboard` layer does not.
        assert_selects(&layers, "/(auth)/dashboard", &["/(auth)"]);
    }

    #[test]
    fn for_path_distinguishes_param_names() {
        let layers = [layer_at("/users/{id}"), layer_at("/users/{user_id}")];
        // Prefix matching compares segments, so `{id}` only wraps endpoints
        // spelled with the same parameter name.
        assert_selects(&layers, "/users/{id}/posts", &["/users/{id}"]);
    }

    // -- Next --

    #[test]
    fn run_invokes_the_route_terminal_when_no_layers_remain() {
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let cx = Cx::default();

        let next = Next::new(&[], Terminal::Route(&route));
        let result = block_on(next.run(&cx, Body::empty()));
        let response = respond(&cx, result);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"route");
    }

    #[test]
    fn run_resolves_the_method_not_allowed_terminal() {
        let mut endpoint = Endpoint::new(&path("/x"), Box::new([]));
        endpoint.insert(Method::GET, RouteIndex::new(0));
        endpoint.insert(Method::POST, RouteIndex::new(1));
        let cx = Cx::default();

        let next = Next::new(&[], Terminal::MethodNotAllowed(&endpoint));
        let result = block_on(next.run(&cx, Body::empty()));
        let response = respond(&cx, result);

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        // The `Allow` header is built from the endpoint's supported methods.
        let allow = response
            .headers()
            .get(http::header::ALLOW)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(allow.contains("GET"), "{allow:?}");
        assert!(allow.contains("POST"), "{allow:?}");
    }

    #[test]
    fn run_walks_layers_in_order_before_the_terminal() {
        let layers: [Arc<dyn Layer>; 2] = [
            Arc::new(LayerFn::new(path("/"), record_a)),
            Arc::new(LayerFn::new(path("/"), record_b)),
        ];
        let route = RouteFn::new(Method::GET, path("/x"), record_route);

        let trace: Arc<Trace> = Arc::new(Mutex::new(Vec::new()));
        let cx = cx_with_trace(trace.clone());

        let next = Next::new(&layers, Terminal::Route(&route));
        block_on(next.run(&cx, Body::empty())).unwrap();

        // The layers run in slice order, then the terminal route.
        assert_eq!(*trace.lock().unwrap(), vec!["a", "b", "route"]);
    }

    #[test]
    fn run_lets_a_layer_short_circuit_without_calling_next() {
        let layers: [Arc<dyn Layer>; 1] = [Arc::new(LayerFn::new(path("/"), short_circuit))];
        // The route would answer "route", but the layer never calls `next.run`.
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let cx = Cx::default();

        let next = Next::new(&layers, Terminal::Route(&route));
        let result = block_on(next.run(&cx, Body::empty()));
        let response = respond(&cx, result);

        assert_eq!(&body_bytes(response)[..], b"short");
    }
}
