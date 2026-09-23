#[cfg(feature = "router")]
use topcoat_router::RouterBuilder;

#[cfg(feature = "router")]
use crate::RuntimeLayer;

/// Marks a router's app context as configured for the browser runtime.
#[derive(Debug, Clone, Copy)]
pub struct RuntimeSetup;

/// Sets up the browser runtime on a [`RouterBuilder`].
#[cfg(feature = "router")]
pub trait RouterBuilderRuntimeExt {
    /// Registers the [`RuntimeLayer`] serving the browser runtime's
    /// requests.
    ///
    /// Call this once when building a router that serves interactive pages,
    /// after the pathless layers the application registers itself. The
    /// runtime layer wraps the layers registered before it, so they see a
    /// page re-run as the plain `GET` it is rewritten into rather than the
    /// runtime's `POST` envelope. Register application procedures and shards
    /// separately, through discovery or explicit registration.
    #[must_use]
    fn runtime(self) -> Self;
}

#[cfg(feature = "router")]
impl RouterBuilderRuntimeExt for RouterBuilder {
    fn runtime(self) -> Self {
        self.layer(RuntimeLayer).app_context(RuntimeSetup)
    }
}
