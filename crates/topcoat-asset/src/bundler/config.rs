use std::{path::PathBuf, sync::mpsc, time::Duration};

use super::{BundleEvent, BundleEvents, BundleSubscriber};

/// The number of assets processed at once when
/// [`parallelism`](BundlerConfig::parallelism) is not set.
pub const DEFAULT_PARALLELISM: usize = 8;

/// The time limit for one download when [`timeout`](BundlerConfig::timeout)
/// is not set.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_mins(1);

/// The time limit for opening a connection when
/// [`connect_timeout`](BundlerConfig::connect_timeout) is not set.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Settings for a [`Bundler`](super::Bundler).
///
/// [`BundlerConfig::new`] starts with every setting at its default. Each
/// builder method changes one setting.
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
    /// Creates a config with every setting at its default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the directory that remote assets are downloaded into.
    ///
    /// A downloaded file stays in this directory and is reused by later
    /// runs. When no directory is set, the bundler uses
    /// `<target>/topcoat/cache/asset` of the Cargo build that runs it, which
    /// only works inside a build script. Outside a build script, set the
    /// directory explicitly, or [`Bundler::new`](super::Bundler::new) panics.
    #[must_use]
    pub fn cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.cache_dir = Some(cache_dir);
        self
    }

    /// Sets how many assets are processed at the same time.
    ///
    /// Each worker thread reads or downloads one asset at a time, hashes it,
    /// and writes it into the bundle. So this is also the maximum number of
    /// downloads in flight. Defaults to [`DEFAULT_PARALLELISM`]. A value of
    /// `0` is treated as `1`.
    #[must_use]
    pub fn parallelism(mut self, parallelism: usize) -> Self {
        self.parallelism = Some(parallelism);
        self
    }

    /// Sets the time limit for downloading one remote asset, including
    /// opening the connection. Defaults to [`DEFAULT_TIMEOUT`].
    ///
    /// Ignored when you set an [`agent`](Self::agent), which has its own
    /// timeouts.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the time limit for opening the connection to a remote asset.
    /// Defaults to [`DEFAULT_CONNECT_TIMEOUT`].
    ///
    /// Ignored when you set an [`agent`](Self::agent), which has its own
    /// timeouts.
    #[must_use]
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    /// Downloads remote assets with your own [`ureq::Agent`], for example to
    /// use a proxy, authentication, or custom TLS settings.
    ///
    /// The agent is used as it is, so [`timeout`](Self::timeout) and
    /// [`connect_timeout`](Self::connect_timeout) have no effect. Configure
    /// timeouts on the agent instead.
    #[must_use]
    pub fn agent(mut self, agent: ureq::Agent) -> Self {
        self.agent = Some(agent);
        self
    }

    /// Sends every [`BundleEvent`] to `subscriber`.
    ///
    /// Call this several times to send the events to several subscribers.
    /// Subscribers run on the bundler's worker threads, so keep them cheap.
    #[must_use]
    pub fn subscribe(mut self, subscriber: impl BundleSubscriber) -> Self {
        self.events.push(subscriber);
        self
    }

    /// Sends every [`BundleEvent`] to a channel and returns its receiver.
    ///
    /// Use this instead of [`subscribe`](Self::subscribe) to pull events
    /// rather than receive callbacks. The config and every bundler created
    /// from it hold a sender, so the receiver disconnects once all of them
    /// are dropped.
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

    /// Returns the configured agent, or builds one with this crate's user
    /// agent and the configured timeouts.
    ///
    /// Cloning a [`ureq::Agent`] is cheap and shares its connection pool.
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
