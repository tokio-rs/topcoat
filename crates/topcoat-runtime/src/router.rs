#[cfg(feature = "router")]
use topcoat_router::RouterBuilder;

#[cfg(feature = "router")]
use crate::PageRerunRoute;

/// The app context value marking a router as set up for the browser
/// runtime.
///
/// [`RouterBuilderRuntimeExt::runtime`] registers it, and the script
/// component checks for it, so a page that includes the runtime script on
/// a router missing the runtime's routes fails loudly instead of breaking
/// in the browser.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeSetup;

/// Sets up the browser runtime on a [`RouterBuilder`].
#[cfg(feature = "router")]
pub trait RouterBuilderRuntimeExt {
    /// Mounts the routes the browser runtime talks to on its own, such as
    /// the route a page re-runs through, and marks the router as set up
    /// with [`RuntimeSetup`].
    ///
    /// Every application using the runtime calls this once. The routes
    /// behind the application's own procedures and shards are registered
    /// separately, by discovery or by hand.
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
