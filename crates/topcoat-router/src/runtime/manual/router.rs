use std::{any::Any, sync::Arc};

use axum::{
    response::Html,
    routing::{MethodFilter, get, on},
};
use topcoat_core::runtime::context::State;

use crate::runtime::{CxBody, Layout, Page, Route, finalize, not_found, result_into_response};

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
    routes: Vec<Route>,
    pages: Vec<Page>,
    layouts: Vec<Layout>,

    #[cfg(feature = "asset")]
    assets: topcoat_asset::AssetBundle,
    #[cfg(feature = "runtime")]
    shards: topcoat_runtime::runtime::Shards,
    #[cfg(feature = "runtime")]
    procedures: Vec<crate::runtime::ErasedProcedure>,

    state: State,
}

impl Router {
    /// Creates an empty router with no pages or layouts.
    pub fn new() -> Self {
        let mut state = State::new();
        // Register `()` so APIs generic over an app state type can default to `S = ()`.
        state.register(());
        Self {
            routes: Vec::new(),
            pages: Vec::new(),
            layouts: Vec::new(),
            #[cfg(feature = "asset")]
            assets: topcoat_asset::AssetBundle::empty(),
            #[cfg(feature = "runtime")]
            shards: topcoat_runtime::runtime::Shards::new(),
            #[cfg(feature = "runtime")]
            procedures: Vec::new(),
            state,
        }
    }

    /// Returns `true` if no pages, layouts, routes or other handlers have been registered.
    pub fn is_empty(&self) -> bool {
        [
            self.routes.is_empty(),
            self.pages.is_empty(),
            self.layouts.is_empty(),
            #[cfg(feature = "runtime")]
            self.shards.is_empty(),
            #[cfg(feature = "runtime")]
            self.procedures.is_empty(),
        ]
        .into_iter()
        .all(core::convert::identity)
    }

    /// Registers a [`Page`]. Order doesn't matter — layout matching
    /// is based on path prefixes, not registration order.
    pub fn page(mut self, page: Page) -> Self {
        self.pages.push(page);
        self
    }

    /// Registers a [`Layout`]. A layout applies to every page whose
    /// path starts with the layout's path prefix.
    pub fn layout(mut self, layout: Layout) -> Self {
        self.layouts.push(layout);
        self
    }

    /// Registers a [`Route`], an HTTP API handler bound to a specific path.
    ///
    /// Unlike pages, routes don't render a [`View`](topcoat_view::runtime::View)
    /// and aren't wrapped by layouts — they return a raw response.
    pub fn route(mut self, route: Route) -> Self {
        self.routes.push(route);
        self
    }

    /// Registers an [`AssetBundle`](topcoat_asset::AssetBundle) of files
    /// declared with `asset!`.
    ///
    /// This does two things: it mounts the bundle so its files are served, and
    /// it installs the resolver that turns [`Asset`](topcoat_asset::Asset)
    /// values rendered in a [`View`](topcoat_view::runtime::View) into their
    /// bundled URLs. Without it, rendering a page that references an `Asset`
    /// panics.
    ///
    /// Load the bundle produced by `topcoat dev` or `topcoat asset bundle` with
    /// [`AssetBundle::load()`](topcoat_asset::AssetBundle::load) for the default
    /// location, or
    /// [`AssetBundle::load_dir()`](topcoat_asset::AssetBundle::load_dir) for a
    /// custom one.
    ///
    /// The binary and the asset bundle must come from the same build: if a page
    /// renders an `Asset` that isn't present in the loaded bundle, rendering
    /// panics.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use topcoat::asset::AssetBundle;
    /// use topcoat::router::Router;
    ///
    /// pub fn router() -> Router {
    ///     Router::new()
    ///         .page(about)
    ///         .assets(AssetBundle::load().unwrap())
    /// }
    /// ```
    #[cfg(feature = "asset")]
    pub fn assets(mut self, assets: topcoat_asset::AssetBundle) -> Self {
        self.assets = assets;
        self
    }

    #[cfg(feature = "runtime")]
    pub fn shard(mut self, shard: &'static dyn topcoat_runtime::runtime::DynShard) -> Self {
        self.shards.register(shard);
        self
    }

    #[cfg(feature = "runtime")]
    pub fn procedure(mut self, procedure: impl Into<crate::runtime::ErasedProcedure>) -> Self {
        self.procedures.push(procedure.into());
        self
    }

    /// Auto-registers every annotated [`Page`], [`Layout`], and [`Route`]
    /// across the crate (and its dependencies) instead of listing each one by
    /// hand.
    ///
    /// With the `discover` feature enabled, items annotated with `#[page]`,
    /// `#[layout]`, and `#[route]` are collected at link time.
    /// Calling `discover()` registers all of them at once.
    ///
    /// This also applies to `#[procedure]` and `#[shard]` when the `runtime`
    /// feature is active.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use topcoat::router::Router;
    ///
    /// pub fn router() -> Router {
    ///     Router::new().discover()
    /// }
    /// ```
    #[cfg(feature = "discover")]
    pub fn discover(mut self) -> Self {
        for page in inventory::iter::<Page>().cloned() {
            self = self.page(page);
        }
        for layout in inventory::iter::<Layout>().cloned() {
            self = self.layout(layout);
        }
        for route in inventory::iter::<Route>().cloned() {
            self = self.route(route);
        }

        #[cfg(feature = "runtime")]
        {
            for shard in
                inventory::iter::<&'static dyn topcoat_runtime::runtime::DynShard>().cloned()
            {
                self = self.shard(shard);
            }
            for procedure in inventory::iter::<crate::runtime::ErasedProcedure>().cloned() {
                self = self.procedure(procedure);
            }
        }

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
        let mut axum_router = axum::Router::<Arc<State>>::new();
        #[allow(unused_mut)]
        let mut state = value.state;

        for page in value.pages {
            let path = page.path();
            let mut layouts: Vec<_> = value
                .layouts
                .iter()
                .filter(|layout| path.starts_with(layout.path()))
                .cloned()
                .collect();
            layouts.sort_by_key(|layout| layout.path().len());

            axum_router = axum_router.route(
                &page.path().to_matchit_path(),
                get(async move |CxBody { cx, body }: CxBody| {
                    let result = {
                        let mut render = page.render(&cx, body);
                        for layout in layouts.iter().rev() {
                            render = layout.render(&cx, render);
                        }
                        render.await
                    };
                    finalize(
                        &cx,
                        result_into_response(result.map(|view| Html(view.render(&cx)))),
                    )
                }),
            );
        }

        for route in value.routes {
            axum_router = axum_router.route(
                &route.path().to_matchit_path(),
                on(
                    MethodFilter::try_from(route.method().clone())
                        .unwrap_or_else(|_| panic!("unsupported method {:?}", route.method())),
                    async move |CxBody { cx, body }: CxBody| {
                        finalize(&cx, result_into_response(route.handle(&cx, body).await))
                    },
                ),
            );
        }

        #[cfg(feature = "asset")]
        {
            let assets = value.assets;
            axum_router = axum_router.nest_service(
                "/_topcoat/assets",
                topcoat_asset::ServeAssetBundle::new(&assets),
            );
            let asset_resolver =
                topcoat_asset::AssetResolver::new(Box::new(move |_cx, asset, f| {
                    match assets.get(asset) {
                        Some(asset) => {
                            f.write_str("/_topcoat/assets/");
                            f.write_str(asset.name().to_str().expect("asset had non-UTF8 name"));
                        }
                        None => {
                            panic!("failed to resolve asset {asset:?} in router's asset bundle")
                        }
                    }
                }));

            state.register(asset_resolver);
        }

        #[cfg(feature = "runtime")]
        {
            let mut procedure_router = axum::Router::new();
            for procedure in value.procedures.into_iter() {
                procedure_router = procedure_router.route(
                    &("/".to_owned() + procedure.id().as_str()),
                    axum::routing::post(async move |CxBody { cx, body }: CxBody| {
                        finalize(&cx, result_into_response(procedure.handle(&cx, body).await))
                    }),
                );
            }
            axum_router = axum_router.nest("/_topcoat/procedures", procedure_router);
        }

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

                            finalize(
                                &cx,
                                result_into_response(result.map(|view| Html(view.render(&cx)))),
                            )
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
