use std::{path::PathBuf, sync::mpsc, time::Duration};

use super::{BundleEvent, BundleEvents, BundleSubscriber};

/// Number of assets processed at once when
/// [`parallelism`](BundlerConfig::parallelism) is unset.
pub const DEFAULT_PARALLELISM: usize = 8;

/// Wall-clock limit for a single download when
/// [`timeout`](BundlerConfig::timeout) is unset.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_mins(1);

/// Limit for establishing a connection when
/// [`connect_timeout`](BundlerConfig::connect_timeout) is unset.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Tuning knobs for a [`Bundler`](super::Bundler).
///
/// Every field is optional; [`BundlerConfig::new`] leaves them all at
/// their defaults, and each builder method overrides one of them.
///
/// ```no_run
/// # use std::time::Duration;
/// use topcoat_asset::{BundleEvent, Bundler, BundlerConfig};
///
/// let config = BundlerConfig::new()
///     .parallelism(16)
///     .timeout(Duration::from_secs(30))
///     .subscribe(|event: &BundleEvent| println!("{event}"));
///
/// let bundler = Bundler::new(&config);
/// ```
#[derive(Clone, Default)]
pub struct BundlerConfig {
    cache_dir: Option<PathBuf>,
    parallelism: Option<usize>,
    timeout: Option<Duration>,
    connect_timeout: Option<Duration>,
    agent: Option<ureq::Agent>,
    events: BundleEvents,
}

impl BundlerConfig {
    /// A config with every knob at its default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.cache_dir = Some(cache_dir);
        self
    }

    /// How many assets to process concurrently.
    ///
    /// Each worker handles one asset at a time -- reading or downloading
    /// it, hashing it, and writing it into the bundle -- so this is
    /// effectively the number of downloads in flight. Defaults to
    /// [`DEFAULT_PARALLELISM`]; a value of `0` is treated as `1`.
    #[must_use]
    pub fn parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = Some(parallelism);
        self
    }

    /// Wall-clock limit for downloading a single remote asset,
    /// connection included. Defaults to [`DEFAULT_TIMEOUT`].
    ///
    /// Ignored when a custom [`agent`](Self::agent) is supplied, since
    /// that agent carries its own timeouts.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Limit for establishing the connection to a remote asset.
    ///
    /// Behaves like [`timeout`](Self::timeout), defaulting to
    /// [`DEFAULT_CONNECT_TIMEOUT`].
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Download remote assets through a caller-supplied [`ureq::Agent`]
    /// (for proxies, auth, custom TLS, etc.).
    ///
    /// The agent is used as-is, so it supersedes
    /// [`timeout`](Self::timeout) and
    /// [`connect_timeout`](Self::connect_timeout); configure those on
    /// the agent instead.
    #[must_use]
    pub fn agent(mut self, agent: ureq::Agent) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Report [`BundleEvent`]s to `subscriber`.
    ///
    /// Call this more than once to fan the same events out to several
    /// consumers. Subscribers run on the bundler's worker threads, so
    /// keep them cheap.
    #[must_use]
    pub fn subscribe(mut self, subscriber: impl BundleSubscriber) -> Self {
        self.events.push(subscriber);
        self
    }

    /// Report [`BundleEvent`]s to a channel, returning its receiver.
    ///
    /// A [`subscribe`](Self::subscribe) shorthand for consumers that
    /// would rather pull events than be called back. The receiver
    /// disconnects once the bundler is dropped.
    ///
    /// ```no_run
    /// use topcoat_asset::{Bundler, BundlerConfig};
    ///
    /// let (config, events) = BundlerConfig::new().event_channel();
    /// std::thread::spawn(move || {
    ///     for event in events {
    ///         println!("{event}");
    ///     }
    /// });
    ///
    /// let bundler = Bundler::new(&config);
    /// ```
    #[must_use]
    pub fn event_channel(self) -> (Self, mpsc::Receiver<BundleEvent>) {
        let (sender, receiver) = mpsc::channel();
        let config = self.subscribe(move |event: &BundleEvent| {
            let _ = sender.send(event.clone());
        });
        (config, receiver)
    }

    pub(super) fn resolve_cache_dir(&self) -> PathBuf {
        self.cache_dir.clone().unwrap_or_else(|| {
            topcoat_core::cache::cache_dir("asset")
                .expect("could not find asset cache dir for bundling")
        })
    }

    pub(super) fn resolve_parallelism(&self) -> usize {
        self.parallelism.unwrap_or(DEFAULT_PARALLELISM).max(1)
    }

    /// The configured agent, or one built with this crate's user agent
    /// and the configured timeouts.
    ///
    /// Cloning a [`ureq::Agent`] is cheap and shares its connection
    /// pool, so callers get the agent they supplied.
    pub(super) fn resolve_agent(&self) -> ureq::Agent {
        if let Some(agent) = &self.agent {
            return agent.clone();
        }

        ureq::Agent::config_builder()
            .user_agent(concat!("topcoat-asset/", env!("CARGO_PKG_VERSION")))
            .timeout_global(Some(self.timeout.unwrap_or(DEFAULT_TIMEOUT)))
            .timeout_connect(Some(
                self.connect_timeout.unwrap_or(DEFAULT_CONNECT_TIMEOUT),
            ))
            .build()
            .into()
    }

    pub(super) const fn events(&self) -> &BundleEvents {
        &self.events
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[test]
    fn parallelism_defaults_and_overrides() {
        assert_eq!(
            BundlerConfig::new().resolve_parallelism(),
            DEFAULT_PARALLELISM
        );
        assert_eq!(BundlerConfig::new().parallelism(3).resolve_parallelism(), 3);
    }

    #[test]
    fn no_parallelism_still_leaves_one_worker() {
        assert_eq!(BundlerConfig::new().parallelism(0).resolve_parallelism(), 1);
    }

    #[test]
    fn the_configured_cache_dir_wins() {
        let dir = PathBuf::from("/tmp/topcoat-asset-cache");
        assert_eq!(
            BundlerConfig::new()
                .cache_dir(dir.clone())
                .resolve_cache_dir(),
            dir
        );
    }

    #[test]
    fn the_built_in_agent_identifies_itself() {
        let agent = BundlerConfig::new().resolve_agent();
        let user_agent = format!("{:?}", agent.config().user_agent());
        assert!(
            user_agent.contains("topcoat-asset/"),
            "unexpected user agent: {user_agent}"
        );
    }

    #[test]
    fn a_supplied_agent_is_used_as_is() {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent("custom-agent/1.0")
            .build()
            .into();

        let resolved = BundlerConfig::new()
            .agent(agent)
            .timeout(Duration::from_secs(1))
            .resolve_agent();

        let user_agent = format!("{:?}", resolved.config().user_agent());
        assert!(
            user_agent.contains("custom-agent/1.0"),
            "the supplied agent was replaced: {user_agent}"
        );
    }

    #[test]
    fn subscribers_receive_emitted_events() {
        let count = Arc::new(AtomicUsize::new(0));

        let seen = Arc::clone(&count);
        let config = BundlerConfig::new()
            .subscribe(move |_: &BundleEvent| {
                seen.fetch_add(1, Ordering::Relaxed);
            })
            .subscribe(|_: &BundleEvent| {});

        config.events().emit(&BundleEvent::Scanned { count: 1 });
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn each_event_channel_receives_every_event() {
        let (config, first) = BundlerConfig::new().event_channel();
        let (config, second) = config.event_channel();

        config.events().emit(&BundleEvent::Scanned { count: 7 });
        drop(config);

        let first: Vec<_> = first.into_iter().map(|event| event.to_string()).collect();
        let second: Vec<_> = second.into_iter().map(|event| event.to_string()).collect();
        assert_eq!(first, ["found 7 assets"]);
        assert_eq!(first, second);
    }
}
