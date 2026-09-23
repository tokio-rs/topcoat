#[cfg(feature = "router")]
use topcoat_router::RouterBuilder;

#[cfg(feature = "router")]
use crate::PageRerunRoute;

/// An app context value that marks a router as set up for the browser
/// runtime.
///
/// `RouterBuilderRuntimeExt::runtime` registers it. Code that renders the
/// runtime script can check for it, so that a router missing the runtime's
/// routes fails on the server instead of breaking in the browser.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeSetup;

/// Sets up the browser runtime on a [`RouterBuilder`].
#[cfg(feature = "router")]
pub trait RouterBuilderRuntimeExt {
    /// Mounts the routes the browser runtime calls on its own, such as the
    /// route that re-runs a page, and marks the router as set up with
    /// [`RuntimeSetup`].
    ///
    /// Call this once on every router that serves pages using the runtime.
    /// The routes for the application's own procedures and shards are
    /// registered separately, by discovery or by hand.
    #[must_use]
    fn runtime(self) -> Self;
}

#[cfg(feature = "router")]
impl RouterBuilderRuntimeExt for RouterBuilder {
    fn runtime(self) -> Self {
        self.route(PageRerunRoute::root())
            .route(PageRerunRoute::nested())
            .app_context(RuntimeSetup)
    }
}
