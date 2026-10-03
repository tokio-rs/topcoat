/// Settings for the browser runtime.
///
/// Register it with the router's `runtime` method. Use
/// [`RuntimeConfig::builder`] to customize it, or `RuntimeConfig::default()`
/// to use the default settings.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub(crate) max_runs_per_connection: usize,
}

/// How many connected renders one runtime connection may have at once,
/// unless overridden with [`RuntimeConfigBuilder::max_runs_per_connection`]:
/// 64.
pub const DEFAULT_MAX_RUNS_PER_CONNECTION: usize = 64;

impl RuntimeConfig {
    /// Creates a builder for a runtime configuration.
    #[must_use]
    pub fn builder() -> RuntimeConfigBuilder {
        RuntimeConfigBuilder::default()
    }
}

/// Builds the all-defaults configuration, like [`RuntimeConfig::builder`]
/// with an immediate [`build`](RuntimeConfigBuilder::build).
impl Default for RuntimeConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

/// Assembles a [`RuntimeConfig`]. Created with [`RuntimeConfig::builder`].
#[derive(Debug, Clone)]
pub struct RuntimeConfigBuilder {
    max_runs_per_connection: usize,
}

impl RuntimeConfigBuilder {
    /// Overrides how many connected renders one runtime connection may have
    /// at once.
    ///
    /// The browser keeps one connected render for each live page or shard
    /// that is not inside another one. Once a connection reaches the limit,
    /// the server answers further render requests with
    /// `429 Too Many Requests` until a render finishes or is stopped.
    #[must_use]
    pub fn max_runs_per_connection(mut self, max: usize) -> Self {
        self.max_runs_per_connection = max;
        self
    }

    /// Consumes the builder, returning the finished [`RuntimeConfig`].
    #[must_use]
    pub fn build(self) -> RuntimeConfig {
        RuntimeConfig {
            max_runs_per_connection: self.max_runs_per_connection,
        }
    }
}

impl Default for RuntimeConfigBuilder {
    fn default() -> Self {
        Self {
            max_runs_per_connection: DEFAULT_MAX_RUNS_PER_CONNECTION,
        }
    }
}
