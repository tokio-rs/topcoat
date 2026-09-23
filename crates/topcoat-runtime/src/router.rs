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
    /// Enables page reruns by registering a [`RuntimeLayer`].
    ///
    /// Call this once when building a router that serves interactive pages.
    /// Register your application's pathless layers first so the runtime
    /// layer can rewrite page reruns to `GET` before those layers run.
    /// See [`RuntimeLayer`] for the request format and rewrite behavior.
    ///
    /// Register procedures and shards separately through discovery or
    /// explicit registration.
    #[must_use]
    fn runtime(self) -> Self;
}

#[cfg(feature = "router")]
impl RouterBuilderRuntimeExt for RouterBuilder {
    fn runtime(self) -> Self {
        self.layer(RuntimeLayer).app_context(RuntimeSetup)
    }
}
