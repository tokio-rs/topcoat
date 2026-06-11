use std::{any::Any, sync::Arc};

use axum::{
    response::Html,
    routing::{MethodFilter, get, on},
};
use topcoat_asset::{AssetBundle, AssetResolver, ServeAssetBundle};
use topcoat_core::runtime::context::{MaybeAborted, State, WatchAbort};

use crate::runtime::{
    CxBody, Layout, Layouts, Page, PageWithLayouts, Route, not_found, result_into_response,
};

/// The core routing primitive that collects [`Page`]s, [`Layout`]s, and
/// [`Route`]s, matches layouts to pages by path prefix, and converts into an
/// [`axum::Router`] for serving.
///
/// Pages, layouts, and routes can be registered manually via
/// [`page()`](Self::page), [`layout()`](Self::layout), and
/// [`route()`](Self::route), or auto-discovered with
/// `discover()` (requires the `discover` feature).
///
/// # Examples
///
/// Manual registration:
///
/// ```rust,ignore
/// use topcoat::router::Router;
///
/// pub fn router() -> Router {
///     Router::new()
///         .layout(root_layout)
///         .page(home)
///         .page(about)
///         .route(api_health)
///         .app_state(Database::connect())
/// }
/// ```
///
/// Auto-discovery:
///
/// ```rust,ignore
/// pub fn router() -> Router {
///     Router::new()
///         .discover()
///         .app_state(Database::connect())
/// }
/// ```
#[derive(Default)]
pub struct Router {
    layouts: Layouts,
    page_registered: bool,

    #[cfg(feature = "runtime")]
    shards: topcoat_runtime::runtime::Shards,

    assets: AssetBundle,
    state: State,

    inner: axum::Router<Arc<State>>,
}

impl Router {
    /// Creates an empty router with no pages or layouts.
    pub fn new() -> Self {
        let mut state = State::new();
        // Register `()` so APIs generic over an app state type can default to `S = ()`.
        state.register(());

        Self {
            layouts: Layouts::new(),
            page_registered: false,
            #[cfg(feature = "runtime")]
            shards: topcoat_runtime::runtime::Shards::new(),
            assets: AssetBundle::empty(),
            state,
            inner: axum::Router::new(),
        }
    }

    /// Returns `true` if no pages, layouts, or routes have been registered.
    pub fn is_empty(&self) -> bool {
        !self.inner.has_routes() && self.layouts.is_empty()
    }

    /// Registers a [`Route`], an HTTP API handler bound to a specific path.
    ///
    /// Unlike pages, routes don't render a [`View`](topcoat_view::runtime::View)
    /// and aren't wrapped by layouts — they return a raw response.
    ///
    /// # Panics
    ///
    /// Panics if a route has already been registered for the same path.
    pub fn route(mut self, route: impl Route + Clone) -> Self {
        self.inner = self.inner.route(
            &route.path().to_axum_path(),
            on(
                MethodFilter::try_from(route.method())
                    .unwrap_or_else(|_| panic!("unsupported method {:?}", route.method())),
                async move |CxBody { cx, body }: CxBody| {
                    let result = WatchAbort::new(&cx, route.handle(&cx, body)).await;

                    match result {
                        MaybeAborted::Completed(result) => result_into_response(result),
                        MaybeAborted::Aborted(_value) => {
                            panic!("request was aborted with an unrecognized type");
                        }
                    }
                },
            ),
        );
        self
    }

    /// Registers a [`Page`], wrapping it in every [`Layout`] whose path is a
    /// prefix of the page's path. Layouts must be registered before the pages
    /// they wrap, since each page snapshots its matching layouts here.
    pub fn page(mut self, page: impl Page) -> Self {
        self.page_registered = true;

        let page: Arc<dyn Page> = Arc::new(page);
        let mut layouts: Layouts = self
            .layouts
            .iter()
            .filter(|layout| page.path().starts_with(layout.path()))
            .cloned()
            .collect();
        layouts.sort_by_key(|layout| layout.path().len());

        self.route(PageWithLayouts::new(page, layouts))
    }

    /// Registers a [`Layout`]. A layout applies to every page whose
    /// path starts with the layout's path prefix.
    ///
    /// # Panics
    ///
    /// Panics if a layout is registered after a page. Layouts must be registered first.
    pub fn layout(mut self, layout: impl Layout) -> Self {
        assert!(!self.page_registered, "layouts must be registered before pages");
        self.layouts.push(Arc::new(layout));
        self
    }

    #[cfg(feature = "runtime")]
    pub fn shard(mut self, shard: &'static dyn topcoat_runtime::runtime::DynShard) -> Self {
        self.shards.register(shard);
        self
    }

    /// Registers a [`Procedure`](crate::runtime::Procedure).
    #[cfg(feature = "runtime")]
    pub fn procedure(self, procedure: &'static dyn crate::runtime::Procedure) -> Self {
        self.route(crate::runtime::ProcedureRoute::new(procedure))
    }

    #[cfg(feature = "discover")]
    pub fn discover(mut self) -> Self {
        // Layouts must be registered before pages, since each page snapshots its
        // matching layouts at registration time.
        for layout in inventory::iter::<&'static dyn Layout>() {
            self = self.layout(*layout);
        }
        for page in inventory::iter::<&'static dyn Page>() {
            self = self.page(*page);
        }
        for route in inventory::iter::<&'static dyn Route>() {
            self = self.route(*route);
        }

        #[cfg(feature = "runtime")]
        {
            for shard in
                inventory::iter::<&'static dyn topcoat_runtime::runtime::DynShard>().cloned()
            {
                self = self.shard(shard);
            }
            for procedure in inventory::iter::<&'static dyn crate::runtime::Procedure>() {
                self = self.procedure(*procedure);
            }
        }

        self
    }

    pub fn assets(mut self, assets: AssetBundle) -> Self {
        self.assets = assets;
        self
    }

    /// Registers a unique value that is accessible to every request sent to
    /// this router by its type `T`. The top-level
    /// [`app_state`](topcoat_core::runtime::context::app_state) function can be used to
    /// retrieve a reference to this value via a request context.
    ///
    /// # Panics
    ///
    /// Panics if a state value has already been registered for the same type.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use topcoat::context::{Cx, app_state};
    /// use topcoat::router::Router;
    ///
    /// struct Database { /* ... */ }
    ///
    /// pub fn router() -> Router {
    ///     Router::new()
    ///         .page(user_profile)
    ///         .app_state(Database::connect())
    /// }
    ///
    /// async fn fetch_user(cx: &Cx, id: u64) -> User {
    ///     let db: &Database = app_state(cx);
    ///     db.fetch_user(id).await;
    /// }
    /// ```
    pub fn app_state<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.state.register(value);
        self
    }
}

/// Converts into an [`axum::Router`] by wiring each page to its matching
/// layouts. For each page, all layouts whose path is a prefix of the page's
/// path are nested from innermost (most specific) to outermost.
impl From<Router> for axum::Router {
    fn from(value: Router) -> Self {
        // Pages and routes were registered into `inner` as they were added.
        let mut axum_router = value.inner;
        let mut state = value.state;

        let assets = value.assets;
        axum_router = axum_router.nest_service("/_topcoat/assets", ServeAssetBundle::new(&assets));
        let asset_resolver =
            AssetResolver::new(Box::new(move |_cx, asset, f| match assets.get(asset) {
                Some(asset) => {
                    f.write_str("/_topcoat/assets/");
                    f.write_str(asset.name().to_str().expect("asset had non-UTF8 name"));
                }
                None => panic!("failed to resolve asset {asset:?} in router's asset bundle"),
            }));

        state.register(asset_resolver);

        #[cfg(feature = "runtime")]
        {
            let mut shard_router = axum::Router::new();
            for shard in value.shards {
                #[derive(serde::Deserialize)]
                struct SignalsQuery {
                    signals: String,
                }

                shard_router = shard_router.route(
                    &("/".to_owned() + shard.id().as_str()),
                    get(
                        async |axum::extract::Query(query): axum::extract::Query<SignalsQuery>,
                               CxBody { cx, body: _ }: CxBody| {
                            use topcoat_runtime::runtime::EncodedSignals;

                            let signal_param = query.signals;
                            // todo: handle errors properly

                            let result = shard
                                .dyn_render(&cx, EncodedSignals::new(signal_param))
                                .await;

                            result_into_response(result.map(|view| Html(view.render(&cx))))
                        },
                    ),
                );
            }
            axum_router = axum_router.nest("/_topcoat/shards", shard_router);
        }

        axum_router = axum_router.fallback(async move |CxBody { cx: _, body: _ }: CxBody| {
            axum::response::IntoResponse::into_response(not_found())
        });

        axum_router.with_state(Arc::new(state))
    }
}
