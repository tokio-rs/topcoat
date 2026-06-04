use std::{any::Any, sync::Arc};

use axum::{
    extract::Query,
    response::Html,
    routing::{MethodFilter, get, on},
};
use serde::Deserialize;
use topcoat_asset::{AssetBundle, AssetResolver, ServeAssetBundle};
use topcoat_core::context::{MaybeAborted, State, WatchAbort};
use topcoat_runtime::runtime::{DynShard, EncodedSignals, Shards};

use crate::{CxBody, Layout, Layouts, Page, Pages, Route, Routes, not_found, result_into_response};

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
    pages: Pages,
    layouts: Layouts,
    routes: Routes,

    shards: Shards,

    assets: AssetBundle,
    state: State,
}

impl Router {
    /// Creates an empty router with no pages or layouts.
    pub fn new() -> Self {
        let mut state = State::new();
        // Register `()` so APIs generic over an app state type can default to `S = ()`.
        state.register(());
        Self {
            pages: Pages::new(),
            layouts: Layouts::new(),
            routes: Routes::new(),
            shards: Shards::new(),
            assets: AssetBundle::empty(),
            state,
        }
    }

    /// Returns `true` if no pages or layouts have been registered.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() && self.layouts.is_empty() && self.routes.is_empty()
    }

    /// Registers a [`Page`]. Order doesn't matter — layout matching
    /// is based on path prefixes, not registration order.
    ///
    /// # Panics
    ///
    /// Panics if a page has already been registered for the same path.
    pub fn page(mut self, page: Page) -> Self {
        self.pages.register(page);
        self
    }

    /// Registers a [`Layout`]. A layout applies to every page whose
    /// path starts with the layout's path prefix.
    ///
    /// # Panics
    ///
    /// Panics if a layout has already been registered for the same path.
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layouts.register(layout);
        self
    }

    /// Registers a [`Route`], an HTTP API handler bound to a specific path.
    ///
    /// Unlike pages, routes don't render a [`View`](topcoat_view::runtime::View)
    /// and aren't wrapped by layouts — they return a raw response.
    ///
    /// # Panics
    ///
    /// Panics if a route has already been registered for the same path.
    pub fn route(mut self, route: Route) -> Self {
        self.routes.register(route);
        self
    }

    pub fn shard(mut self, shard: &'static dyn DynShard) -> Self {
        self.shards.register(shard);
        self
    }

    /// Discovers and registers all `#[page]`, `#[layout]`, `#[route]`, and
    /// `#[shard]` items
    /// collected at link time across the crate and its dependencies.
    #[cfg(feature = "discover")]
    pub fn discover(mut self) -> Self {
        use topcoat_runtime::runtime::DynShard;

        for page in inventory::iter::<Page>().cloned() {
            self = self.page(page);
        }
        for layout in inventory::iter::<Layout>().cloned() {
            self = self.layout(layout);
        }
        for route in inventory::iter::<Route>().cloned() {
            self = self.route(route);
        }

        for shard in inventory::iter::<&'static dyn DynShard>().cloned() {
            self = self.shard(shard);
        }

        self
    }

    pub fn assets(mut self, assets: AssetBundle) -> Self {
        self.assets = assets;
        self
    }

    /// Registers a unique value that is accessible to every request sent to
    /// this router by its type `T`. The top-level [`app_state`](topcoat_core::context::app_state)
    /// function can be used to retrieve a reference to this value via a request context.
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
        let mut axum_router = axum::Router::<Arc<State>>::new();
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

        for page in value.pages {
            let mut layouts: Vec<_> = value.layouts.for_path(page.path()).cloned().collect();
            layouts.sort_by_key(|layout| layout.path().len());

            axum_router = axum_router.route(
                &page.path().to_axum_path(),
                get(async move |CxBody { cx, body }: CxBody| {
                    let result = WatchAbort::new(&cx, async {
                        let mut render = page.render(&cx, body);
                        for layout in layouts.iter().rev() {
                            render = layout.render(&cx, render);
                        }
                        render.await
                    })
                    .await;

                    match result {
                        MaybeAborted::Completed(result) => {
                            result_into_response(result.map(|view| Html(view.render(&cx))))
                        }
                        MaybeAborted::Aborted(_value) => {
                            panic!("request was aborted with an unrecognized type");
                        }
                    }
                }),
            );
        }

        for route in value.routes {
            axum_router = axum_router.route(
                &route.path().to_axum_path(),
                on(
                    MethodFilter::try_from(route.method().clone())
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
        }

        let mut shard_router = axum::Router::new();
        for shard in value.shards {
            #[derive(Deserialize)]
            struct SignalsQuery {
                signals: String,
            }

            shard_router = shard_router.route(
                &("/".to_owned() + shard.id().as_str()),
                get(
                    async |Query(query): Query<SignalsQuery>, CxBody { cx, body: _ }: CxBody| {
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

        axum_router = axum_router.fallback(async move |CxBody { cx: _, body: _ }: CxBody| {
            axum::response::IntoResponse::into_response(not_found())
        });

        axum_router.with_state(Arc::new(state))
    }
}
