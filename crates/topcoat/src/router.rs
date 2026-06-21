//! Routing primitives for pages, layouts, layers, API routes, and request/response bodies.
//!
//! A [`Router`] is the finalized routing table that dispatches incoming requests. Build one with
//! [`Router::builder`], register handlers on the [`RouterBuilder`], call
//! [`build`](RouterBuilder::build), then pass the router to [`crate::start`].
//!
//! ## Handlers
//!
//! - [`#[page]`](page) declares a page that renders a [`View`](crate::view::View).
//! - [`#[layout]`](layout) declares a layout that wraps matching pages by path prefix.
//! - [`#[layer]`](layer) declares a request-processing layer around matching routes.
//! - [`#[route]`](route) declares an API route for an HTTP method and path.
//!
//! Explicit path strings use Topcoat's [`Path`] syntax: static segments like `/users`, dynamic
//! parameters like `/users/{id}`, wildcard tails like `/docs/{*path}`, and group segments like
//! `/(marketing)/pricing` that participate in layout and layer matching but are stripped from the
//! served URL.
//!
//! ## Manual registration
//!
//! ```rust,ignore
//! use topcoat::{
//!     Result,
//!     router::{Router, Slot, layout, page, route},
//!     view::view,
//! };
//!
//! #[layout("/")]
//! async fn root_layout(slot: Slot<'_>) -> Result {
//!     view! { <html><body>(slot.await?)</body></html> }
//! }
//!
//! #[page("/")]
//! async fn home() -> Result {
//!     view! { <h1>"Home"</h1> }
//! }
//!
//! #[route(GET "/api/health")]
//! async fn health() -> Result<&'static str> {
//!     Ok("ok")
//! }
//!
//! pub fn router() -> Router {
//!     Router::builder()
//!         .layout(root_layout)
//!         .page(home)
//!         .route(health)
//!         .build()
//! }
//! ```
//!
//! ## Auto-discovery
//!
//! With the `discover` feature enabled, [`RouterBuilderDiscoverExt::discover`] collects every
//! [`#[page]`](page), [`#[layout]`](layout), [`#[layer]`](layer), and [`#[route]`](route)
//! registered at link time:
//!
//! ```rust,ignore
//! use topcoat::router::{Router, RouterBuilderDiscoverExt};
//!
//! pub fn router() -> Router {
//!     Router::builder().discover().build()
//! }
//! ```
//!
//! For file-system-shaped routing, call [`module_router!`] from the root module of your route
//! tree. It returns a [`RouterBuilder`], so you can keep chaining builder extensions before
//! `.build()`.
pub use topcoat_router::runtime::*;
pub use topcoat_router_macro::*;

#[cfg(all(feature = "discover", feature = "runtime"))]
use topcoat_runtime::runtime::RouterBuilderProcedureExt;

#[cfg(feature = "discover")]
pub trait RouterBuilderDiscoverExt {
    fn discover(self) -> Self;
}

#[cfg(feature = "discover")]
impl RouterBuilderDiscoverExt for RouterBuilder {
    fn discover(mut self) -> Self {
        self = self.discover_routes();
        self = self.discover_pages();
        self = self.discover_layouts();
        self = self.discover_layers();
        #[cfg(feature = "runtime")]
        {
            self = self.discover_procedures();
        }
        self
    }
}
