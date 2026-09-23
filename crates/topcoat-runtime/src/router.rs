#[cfg(feature = "router")]
use topcoat_router::RouterBuilder;

#[cfg(feature = "router")]
use crate::PageRerunRoute;

/// Marks a router's app context as configured for the browser runtime.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeSetup;

/// Sets up the browser runtime on a [`RouterBuilder`].
#[cfg(feature = "router")]
pub trait RouterBuilderRuntimeExt {
    /// Registers the browser runtime's routes.
    ///
    /// Call this once when building a router that serves interactive pages.
    /// Register application procedures and shards separately, through
    /// discovery or explicit registration.
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
