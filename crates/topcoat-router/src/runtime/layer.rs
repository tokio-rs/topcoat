use std::borrow::Cow;
use std::ops::Index;
use std::pin::Pin;

use topcoat_core::runtime::{context::CxBuilder, error::Result};

use crate::runtime::{Body, Endpoint, Path, Response, Route, method_not_allowed, not_found};

/// The future returned by [`Layer::handle`] and [`Next::run`]: a boxed, `Send`
/// future borrowing the chain and the request context.
pub type LayerFuture<'a> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'a>>;

/// A request-processing layer that wraps the routes nested under its path,
/// similar to a tower middleware.
///
/// A layer wraps every matched route whose path begins with the layer's path
/// (the same prefix rule as layouts), so a layer at `/admin` wraps only routes
/// under `/admin`, while a layer at `/` wraps everything. Each layer receives a
/// mutable [`CxBuilder`] and the request [`Body`], plus a [`Next`] representing
/// the rest of the chain. A layer typically registers request-scoped values on
/// the context, calls [`Next::run`] to invoke the inner layers and ultimately
/// the route, then inspects or modifies the [`Response`].
///
/// When several layers match a route they nest from least-specific (outermost)
/// to most-specific (innermost), like layouts.
///
/// Register layers with [`RouterBuilder::layer`](crate::runtime::RouterBuilder::layer).
///
/// # Examples
///
/// ```rust
/// use std::borrow::Cow;
/// use topcoat::context::CxBuilder;
/// use topcoat::router::{Body, Layer, LayerFuture, Next, Path};
///
/// struct Timing;
///
/// impl Layer for Timing {
///     fn path(&self) -> &Path {
///         Path::new("/")
///     }
///
///     fn handle<'a>(
///         &'a self,
///         cx: &'a mut CxBuilder,
///         body: Body,
///         next: Next<'a>,
///     ) -> LayerFuture<'a> {
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
    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a>;
}

/// The handler function backing a [`LayerFn`].
pub type LayerHandlerFn =
    for<'a> fn(cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a>;

/// A [`Layer`] backed by a plain handler function.
///
/// Created either manually via `#[layer("/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a
/// [`RouterBuilder`](crate::runtime::RouterBuilder) with
/// [`layer`](crate::runtime::RouterBuilder::layer).
#[derive(Debug, Clone)]
pub struct LayerFn {
    /// The URL path prefix whose routes this layer wraps.
    path: Cow<'static, Path>,
    /// The handler function that wraps the inner chain.
    handle: LayerHandlerFn,
}

impl LayerFn {
    /// Creates a new layer with an explicit path prefix and handler function.
    pub const fn new(path: Cow<'static, Path>, handle: LayerHandlerFn) -> Self {
        Self { path, handle }
    }
}

impl Layer for LayerFn {
    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'a>(&'a self, cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        (self.handle)(cx, body, next)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(LayerFn);

/// The identifier of a [`Layer`] registered on a router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LayerId(usize);

/// The layers registered on a router, in registration order, indexed by
/// [`LayerId`].
///
/// Layers are [`push`](Self::push)ed as the router is built, then only queried:
/// [`for_path`](Self::for_path) selects the layers wrapping a request path, and
/// indexing by [`LayerId`] resolves a selected id back to its layer.
#[derive(Default)]
pub(crate) struct Layers {
    layers: Vec<Box<dyn Layer>>,
}

impl Layers {
    /// Registers `layer`, returning the [`LayerId`] that now identifies it.
    pub(crate) fn push(&mut self, layer: Box<dyn Layer>) -> LayerId {
        let id = LayerId(self.layers.len());
        self.layers.push(layer);
        id
    }

    /// Selects the layers whose path matches the URL `path`, ordered least- to
    /// most-specific so the outermost layer runs first. Among layers that share a
    /// path, the most recently registered runs first.
    pub(crate) fn match_path(&self, path: &str) -> Vec<LayerId> {
        let mut ids: Vec<LayerId> = (0..self.layers.len())
            .map(LayerId)
            .filter(|id| self[*id].path().matches_start(path))
            .rev()
            .collect();
        ids.sort_by_key(|id| self[*id].path().len());
        ids
    }

    /// Selects the layers whose start with the endpoint `path`, ordered least- to
    /// most-specific so the outermost layer runs first. Among layers that share a
    /// path, the most recently registered runs first.
    pub(crate) fn for_endpoint(&self, path: &Path) -> Vec<LayerId> {
        let mut ids: Vec<LayerId> = (0..self.layers.len())
            .map(LayerId)
            .filter(|id| path.starts_with(self[*id].path()))
            .rev()
            .collect();
        ids.sort_by_key(|id| self[*id].path().len());
        ids
    }
}

impl Index<LayerId> for Layers {
    type Output = dyn Layer;

    fn index(&self, LayerId(index): LayerId) -> &Self::Output {
        &*self.layers[index]
    }
}

/// What a [`Next`] chain runs once its layers are exhausted.
///
/// The layers wrapping a path are the same whether or not the request resolves
/// to a route, so 404 and 405 responses flow through them too: a layer sees a
/// matched route handler's result, or the not-found / method-not-allowed error,
/// uniformly as the `Result` returned by [`Next::run`].
#[derive(Clone, Copy)]
pub(crate) enum Terminal<'a> {
    /// A matched route handles the request.
    Route(&'a dyn Route),
    /// No route matched the path; the chain resolves to a not-found error.
    NotFound,
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
    /// The router's full layer table, indexed by the ids in `indices`.
    layers: &'a Layers,
    /// The layers wrapping this request, as ids into `layers`, ordered from
    /// least- to most-specific so the outermost layer runs first.
    indices: &'a [LayerId],
    /// What runs once the layers are exhausted.
    terminal: Terminal<'a>,
}

impl<'a> Next<'a> {
    /// Creates a chain that runs `indices` (in order) into `layers`, then
    /// `terminal`.
    ///
    /// `indices` must be ordered from least- to most-specific (ascending path
    /// length), so the outermost layer runs first.
    pub(crate) fn new(layers: &'a Layers, indices: &'a [LayerId], terminal: Terminal<'a>) -> Self {
        Self {
            layers,
            indices,
            terminal,
        }
    }

    /// Runs the next layer in the chain, or the terminal handler once no layers
    /// remain.
    pub fn run(self, cx: &'a mut CxBuilder, body: Body) -> LayerFuture<'a> {
        match self.indices.split_first() {
            Some((&id, rest)) => self.layers[id].handle(
                cx,
                body,
                Next {
                    indices: rest,
                    ..self
                },
            ),
            None => match self.terminal {
                Terminal::Route(route) => route.handle(cx, body),
                Terminal::NotFound => Box::pin(async move { Err(not_found().into()) }),
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
    use std::future::Future;
    use std::sync::{Arc, Mutex};

    use http::StatusCode;
    use topcoat_core::runtime::context::{ContextMap, Cx, CxBuilder, app_context};

    use super::*;
    use crate::runtime::{Bytes, IntoResponse, Method, RouteFn, RouteFuture, respond, to_bytes};

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
    fn layer_at(p: &'static str) -> Box<dyn Layer> {
        Box::new(LayerFn::new(path(p), noop_layer))
    }

    fn noop_layer<'a>(cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
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

    fn cx_with_trace(trace: Arc<Trace>) -> CxBuilder {
        let mut app = ContextMap::new();
        app.insert(trace);
        CxBuilder::new(Arc::new(app))
    }

    fn record_a<'a>(cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("a");
            next.run(cx, body).await
        })
    }

    fn record_b<'a>(cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("b");
            next.run(cx, body).await
        })
    }

    /// A layer that answers the request itself, without invoking `next`.
    fn short_circuit<'a>(cx: &'a mut CxBuilder, _body: Body, _next: Next<'a>) -> LayerFuture<'a> {
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

    // -- Layers --

    #[test]
    fn push_assigns_sequential_ids() {
        let mut layers = Layers::default();
        assert_eq!(layers.push(layer_at("/")), LayerId(0));
        assert_eq!(layers.push(layer_at("/admin")), LayerId(1));
        assert_eq!(layers.push(layer_at("/admin/x")), LayerId(2));
    }

    #[test]
    fn index_resolves_an_id_to_its_layer() {
        let mut layers = Layers::default();
        let root = layers.push(layer_at("/"));
        let admin = layers.push(layer_at("/admin"));
        assert_eq!(layers[root].path(), Path::new("/"));
        assert_eq!(layers[admin].path(), Path::new("/admin"));
    }

    #[test]
    fn match_path_orders_least_to_most_specific() {
        let mut layers = Layers::default();
        // Registered most-specific first, to show the ordering follows path
        // length rather than registration order.
        let admin = layers.push(layer_at("/admin"));
        let root = layers.push(layer_at("/"));
        assert_eq!(layers.match_path("/admin/x"), vec![root, admin]);
    }

    #[test]
    fn match_path_excludes_layers_that_do_not_wrap_the_url() {
        let mut layers = Layers::default();
        let _public = layers.push(layer_at("/public"));
        let admin = layers.push(layer_at("/admin"));
        assert_eq!(layers.match_path("/admin/x"), vec![admin]);
    }

    #[test]
    fn match_path_runs_most_recent_of_a_shared_path_first() {
        let mut layers = Layers::default();
        let first = layers.push(layer_at("/"));
        let second = layers.push(layer_at("/"));
        // Among layers sharing a path, the most recently registered runs first.
        assert_eq!(layers.match_path("/x"), vec![second, first]);
    }

    #[test]
    fn match_path_selects_nothing_when_no_layers_match() {
        let mut layers = Layers::default();
        let _admin = layers.push(layer_at("/admin"));
        assert!(layers.match_path("/public").is_empty());
    }

    #[test]
    fn for_endpoint_orders_prefix_layers_least_to_most_specific() {
        let mut layers = Layers::default();
        let root = layers.push(layer_at("/"));
        let users = layers.push(layer_at("/users"));
        let _posts = layers.push(layer_at("/posts"));
        // The route at /users/{id} is wrapped by the root and /users layers, in
        // that order; the /posts layer does not prefix it.
        assert_eq!(
            layers.for_endpoint(Path::new("/users/{id}")),
            vec![root, users],
        );
    }

    #[test]
    fn for_endpoint_runs_most_recent_of_a_shared_path_first() {
        let mut layers = Layers::default();
        let first = layers.push(layer_at("/admin"));
        let second = layers.push(layer_at("/admin"));
        assert_eq!(
            layers.for_endpoint(Path::new("/admin/users")),
            vec![second, first],
        );
    }

    // -- Next --

    #[test]
    fn run_invokes_the_route_terminal_when_no_layers_remain() {
        let layers = Layers::default();
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let mut cx = CxBuilder::default();

        let indices: &[LayerId] = &[];
        let next = Next::new(&layers, indices, Terminal::Route(&route));
        let result = block_on(next.run(&mut cx, Body::empty()));
        let response = respond(&cx, result);

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"route");
    }

    #[test]
    fn run_resolves_the_not_found_terminal() {
        let layers = Layers::default();
        let mut cx = CxBuilder::default();

        let indices: &[LayerId] = &[];
        let next = Next::new(&layers, indices, Terminal::NotFound);
        let result = block_on(next.run(&mut cx, Body::empty()));
        let response = respond(&cx, result);

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn run_resolves_the_method_not_allowed_terminal() {
        let layers = Layers::default();
        let no_params: Box<[Arc<str>]> = Box::new([]);
        let no_layers: Box<[LayerId]> = Box::new([]);
        let mut endpoint = Endpoint::new(no_params, no_layers);
        endpoint.insert(Method::GET, 0);
        endpoint.insert(Method::POST, 1);
        let mut cx = CxBuilder::default();

        let indices: &[LayerId] = &[];
        let next = Next::new(&layers, indices, Terminal::MethodNotAllowed(&endpoint));
        let result = block_on(next.run(&mut cx, Body::empty()));
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
        let mut layers = Layers::default();
        let a = layers.push(Box::new(LayerFn::new(path("/"), record_a)));
        let b = layers.push(Box::new(LayerFn::new(path("/"), record_b)));
        let indices = [a, b];
        let route = RouteFn::new(Method::GET, path("/x"), record_route);

        let trace: Arc<Trace> = Arc::new(Mutex::new(Vec::new()));
        let mut cx = cx_with_trace(trace.clone());

        let next = Next::new(&layers, &indices, Terminal::Route(&route));
        block_on(next.run(&mut cx, Body::empty())).unwrap();

        // The layers run in `indices` order, then the terminal route.
        assert_eq!(*trace.lock().unwrap(), vec!["a", "b", "route"]);
    }

    #[test]
    fn run_lets_a_layer_short_circuit_without_calling_next() {
        let mut layers = Layers::default();
        let stop = layers.push(Box::new(LayerFn::new(path("/"), short_circuit)));
        let indices = [stop];
        // The route would answer "route", but the layer never calls `next.run`.
        let route = RouteFn::new(Method::GET, path("/x"), say_route);
        let mut cx = CxBuilder::default();

        let next = Next::new(&layers, &indices, Terminal::Route(&route));
        let result = block_on(next.run(&mut cx, Body::empty()));
        let response = respond(&cx, result);

        assert_eq!(&body_bytes(response)[..], b"short");
    }
}
