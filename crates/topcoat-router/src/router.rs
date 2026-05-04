use std::{any::Any, sync::Arc};

use axum::{
    body::Body,
    extract::{RawPathParams, State},
    response::IntoResponse,
    routing::get,
};
use http::Request;
use topcoat_core::context::{AppState, MaybeAborted, scope_context};

use crate::{Layout, Layouts, Page, Pages};

/// The core routing primitive that collects [`Page`]s and [`Layout`]s,
/// matches layouts to pages by path prefix, and converts into an
/// [`axum::Router`] for serving.
///
/// Pages and layouts can be registered manually via [`page()`](Self::page)
/// and [`layout()`](Self::layout), or auto-discovered with
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
/// }
/// ```
///
/// Auto-discovery:
///
/// ```rust,ignore
/// pub fn router() -> Router {
///     Router::new().discover()
/// }
/// ```
#[derive(Default)]
pub struct Router {
    pages: Pages,
    layouts: Layouts,
    state: AppState,
}

impl Router {
    /// Creates an empty router with no pages or layouts.
    pub fn new() -> Self {
        Self {
            pages: Pages::new(),
            layouts: Layouts::new(),
            state: AppState::new(),
        }
    }

    /// Returns `true` if no pages or layouts have been registered.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() && self.layouts.is_empty()
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

    pub fn app_state<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.state.register(value);
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
}

/// Converts into an [`axum::Router`] by wiring each page to its matching
/// layouts. For each page, all layouts whose path is a prefix of the page's
/// path are nested from innermost (most specific) to outermost.
impl From<Router> for axum::Router {
    fn from(value: Router) -> Self {
        let mut result = axum::Router::<Arc<AppState>>::new();

        for page in value.pages {
            let mut layouts: Vec<_> = value.layouts.for_path(page.path()).cloned().collect();
            layouts.sort_by_key(|layout| layout.path().len());

            result = result.route(
                &page.path().to_axum_path(),
                get(
                    async move |State(state): State<Arc<AppState>>,
                                params: RawPathParams,
                                request: Request<Body>| {
                        let mut render = page.render();
                        for layout in layouts.iter().rev() {
                            render = layout.render(render);
                        }

                        let (mut parts, _body) = request.into_parts();
                        parts.extensions.insert(Arc::new(params));
                        match scope_context(state, parts, render).await {
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

        result.with_state(Arc::new(value.state))
    }
}
