#[cfg(feature = "router")]
use topcoat_router::RouterBuilder;

#[cfg(feature = "router")]
use crate::{MaxRunsPerConnection, PrefetchMode, RuntimeLayer};

/// A context marker indicating that the browser runtime has been set up.
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

    /// Chooses when links start loading a page before the user opens it.
    ///
    /// Stores the router default returned by [`prefetch_mode`](crate::prefetch_mode).
    /// Override it in a child context with `Cx::with(PrefetchMode::...)`,
    /// or for one link with its `prefetch` argument.
    ///
    /// ```rust
    /// use topcoat_router::Router;
    /// use topcoat_runtime::{PrefetchMode, RouterBuilderRuntimeExt};
    ///
    /// let router = Router::builder()
    ///     .runtime()
    ///     .prefetch(PrefetchMode::Viewport)
    ///     .build();
    /// ```
    #[must_use]
    fn prefetch(self, mode: PrefetchMode) -> Self;

    /// Limits the number of renders running at once on a runtime connection.
    /// The default limit is 64.
    ///
    /// Each connected page or shard has a render, unless it is inside another
    /// connected page or shard. At the limit, new render requests receive
    /// `429 Too Many Requests`. They can be accepted again once a render
    /// finishes or stops.
    ///
    /// ```rust
    /// use topcoat_router::Router;
    /// use topcoat_runtime::RouterBuilderRuntimeExt;
    ///
    /// let router = Router::builder()
    ///     .runtime()
    ///     .max_runs_per_connection(128)
    ///     .build();
    /// ```
    #[must_use]
    fn max_runs_per_connection(self, max: usize) -> Self;
}

#[cfg(feature = "router")]
impl RouterBuilderRuntimeExt for RouterBuilder {
    fn runtime(self) -> Self {
        self.layer(RuntimeLayer).with(RuntimeSetup)
    }

    fn prefetch(self, mode: PrefetchMode) -> Self {
        self.with(mode)
    }

    fn max_runs_per_connection(self, max: usize) -> Self {
        self.with(MaxRunsPerConnection(max))
    }
}
