use std::{any::Any, sync::Arc};

use axum::{
    body::Body,
    extract::{self, RawPathParams},
    response::IntoResponse,
    routing::get,
};
use http::Request;
use topcoat_asset::{AssetBundle, AssetFragmentResolver, ServeAssetBundle};
use topcoat_core::context::{Cx, MaybeAborted, State, WatchAbort};

use crate::{Layout, Layouts, Page, Pages, Route, Routes};

/// The core routing primitive that collects [`Page`]s, [`Layout`]s, and
/// [`Route`]s, matches layouts to pages by path prefix, and converts into an
/// [`axum::Router`] for serving.
///
/// Pages, layouts, and routes can be registered manually via
/// [`page()`](Self::page), [`layout()`](Self::layout), and
/// [`route()`](Self::route), or auto-discovered with
/// [`discover()`](Self::discover) (requires the `discover` feature).
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

    /// Discovers and registers all `#[page]` and `#[layout]` items
    /// collected at link time across the crate and its dependencies.
    #[cfg(feature = "discover")]
    pub fn discover(mut self) -> Self {
        for page in inventory::iter::<Page>().cloned() {
            self = self.page(page);
        }
        for layout in inventory::iter::<Layout>().cloned() {
            self = self.layout(layout);
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
        let mut result = axum::Router::<Arc<State>>::new();
        let mut state = value.state;

        let assets = value.assets;
        result = result.nest_service("/_topcoat/assets", ServeAssetBundle::new(&assets));
        let asset_resolver =
            AssetFragmentResolver::new(Box::new(move |_cx, asset, f| match assets.get(asset) {
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

            result = result.route(
                &page.path().to_axum_path(),
                get(
                    async move |extract::State(app_state): extract::State<Arc<State>>,
                                params: RawPathParams,
                                request: Request<Body>| {
                        let (parts, _body) = request.into_parts();

                        let mut request_state = State::new();
                        request_state.register(parts);
                        request_state.register(params);

                        let cx = Cx::new(app_state, request_state);

                        let result = WatchAbort::new(&cx, async {
                            let mut render = page.render(&cx);
                            for layout in layouts.iter().rev() {
                                render = layout.render(&cx, render);
                            }
                            render.await
                        })
                        .await;

                        match result {
                            MaybeAborted::Completed(value) => value.into_response(),
                            MaybeAborted::Aborted(value) => {
                                if let Ok(redirect) = value.downcast::<axum::response::Redirect>() {
                                    return redirect.into_response();
                                }

                                panic!("request was aborted with an unrecognized type");
                            }
                        }
                    },
                ),
            );
        }

        result.with_state(Arc::new(state))
    }
}
