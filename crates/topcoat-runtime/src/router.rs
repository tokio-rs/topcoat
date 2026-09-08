use topcoat_router::RouterBuilder;

use crate::PageRerunRoute;

/// Sets up the browser runtime on a [`RouterBuilder`].
pub trait RouterBuilderRuntimeExt {
    /// Mounts the routes the browser runtime talks to on its own, such as
    /// the route a page re-runs through.
    ///
    /// Every application using the runtime calls this once. The routes
    /// behind the application's own procedures and shards are registered
    /// separately, by discovery or by hand.
    #[must_use]
    fn runtime(self) -> Self;
}

impl RouterBuilderRuntimeExt for RouterBuilder {
    fn runtime(self) -> Self {
        self.route(PageRerunRoute::root())
            .route(PageRerunRoute::nested())
    }
}
