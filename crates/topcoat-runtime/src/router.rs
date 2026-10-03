#[cfg(feature = "router")]
use topcoat_router::RouterBuilder;

#[cfg(feature = "router")]
use crate::{PrefetchMode, RuntimeConfig, RuntimeLayer};

/// Sets up the browser runtime on a [`RouterBuilder`].
#[cfg(feature = "router")]
pub trait RouterBuilderRuntimeExt {
    /// Registers the runtime `config` on the app context and enables page
    /// reruns by registering a [`RuntimeLayer`].
    ///
    /// Call this once when building a router that serves interactive pages.
    /// Register your application's pathless layers first so the runtime
    /// layer can rewrite page reruns to `GET` before those layers run.
    /// See [`RuntimeLayer`] for the request format and rewrite behavior.
    ///
    /// Register procedures and shards separately through discovery or
    /// explicit registration.
    ///
    /// ```rust
    /// use topcoat_router::Router;
    /// use topcoat_runtime::{RouterBuilderRuntimeExt, RuntimeConfig};
    ///
    /// let router = Router::builder()
    ///     .runtime(
    ///         RuntimeConfig::builder()
    ///             .max_runs_per_connection(128)
    ///             .build(),
    ///     )
    ///     .build();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the app context already contains a [`RuntimeConfig`].
    #[must_use]
    #[track_caller]
    fn runtime(self, config: RuntimeConfig) -> Self;

    /// Sets when links in this router load their pages ahead of time.
    ///
    /// This sets the app context value read by
    /// [`prefetch_mode`](crate::prefetch_mode). You can override it with a
    /// `PrefetchMode` in the request context or with a link's `prefetch`
    /// argument.
    ///
    /// ```rust
    /// use topcoat_router::Router;
    /// use topcoat_runtime::{PrefetchMode, RouterBuilderRuntimeExt, RuntimeConfig};
    ///
    /// let router = Router::builder()
    ///     .runtime(RuntimeConfig::default())
    ///     .prefetch(PrefetchMode::Viewport)
    ///     .build();
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the app context already contains a `PrefetchMode`.
    #[must_use]
    #[track_caller]
    fn prefetch(self, mode: PrefetchMode) -> Self;
}

#[cfg(feature = "router")]
impl RouterBuilderRuntimeExt for RouterBuilder {
    #[track_caller]
    fn runtime(self, config: RuntimeConfig) -> Self {
        self.layer(RuntimeLayer).app_context(config)
    }

    #[track_caller]
    fn prefetch(self, mode: PrefetchMode) -> Self {
        self.app_context(mode)
    }
}
