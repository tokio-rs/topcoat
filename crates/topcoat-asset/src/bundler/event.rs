use std::{fmt, path::PathBuf, sync::Arc};

use http::Uri;

use crate::AssetId;

/// A step the [`Bundler`](super::Bundler) took while building a bundle.
///
/// Events are reported as they happen, from the worker thread that did the
/// work, so their order can vary between runs. On a successful run,
/// [`Scanned`](Self::Scanned) always comes first and
/// [`Finished`](Self::Finished) always comes last. The [`Display`](fmt::Display)
/// implementation formats an event as a short log line.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BundleEvent {
    /// The binary was scanned.
    Scanned {
        /// The number of asset declarations found.
        count: usize,
    },
    /// A remote asset was found in the download cache.
    CacheHit {
        /// The URL of the asset.
        uri: Uri,
        /// The path of the cached file.
        path: PathBuf,
    },
    /// A remote asset is not cached and its download starts now.
    DownloadStarted {
        /// The URL of the asset.
        uri: Uri,
    },
    /// A remote asset was downloaded into the cache.
    Downloaded {
        /// The URL of the asset.
        uri: Uri,
        /// The path of the cached file.
        path: PathBuf,
        /// The size of the downloaded file.
        bytes: u64,
    },
    /// An asset was written into the bundle directory.
    Bundled {
        /// The ID of the asset.
        id: AssetId,
        /// The bundled filename.
        file: String,
        /// The size of the file.
        bytes: usize,
    },
    /// An asset was already in the bundle directory with the same contents,
    /// so it was not written again.
    Unchanged {
        /// The ID of the asset.
        id: AssetId,
        /// The bundled filename.
        file: String,
    },
    /// A file that is no longer declared was deleted from the bundle
    /// directory.
    Removed {
        /// The deleted filename.
        file: String,
    },
    /// Every asset was processed and the manifest was written.
    Finished {
        /// The number of files written.
        bundled: usize,
        /// The number of files that were already up to date.
        unchanged: usize,
        /// The number of files deleted.
        removed: usize,
    },
}

impl fmt::Display for BundleEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scanned { count } => write!(f, "found {count} assets"),
            Self::CacheHit { uri, .. } => write!(f, "cached {uri}"),
            Self::DownloadStarted { uri } => write!(f, "downloading {uri}"),
            Self::Downloaded { uri, bytes, .. } => {
                write!(f, "downloaded {uri} ({bytes} bytes)")
            }
            Self::Bundled { file, bytes, .. } => write!(f, "bundled {file} ({bytes} bytes)"),
            Self::Unchanged { file, .. } => write!(f, "unchanged {file}"),
            Self::Removed { file } => write!(f, "removed {file}"),
            Self::Finished {
                bundled,
                unchanged,
                removed,
            } => write!(
                f,
                "bundled {bundled} assets ({unchanged} unchanged, {removed} removed)"
            ),
        }
    }
}

/// A receiver of [`BundleEvent`]s, registered with
/// [`BundlerConfig::subscribe`](super::BundlerConfig::subscribe).
///
/// Every `Fn(&BundleEvent) + Send + Sync + 'static` closure implements this
/// trait, so you rarely need to implement it yourself. Subscribers run on
/// the bundler's worker threads and block them while they run. Keep them
/// cheap, and send slow work to a channel or another thread.
pub trait BundleSubscriber: Send + Sync + 'static {
    /// Handles one event.
    fn handle(&self, event: &BundleEvent);
}

impl<F> BundleSubscriber for F
where
    F: Fn(&BundleEvent) + Send + Sync + 'static,
{
    fn handle(&self, event: &BundleEvent) {
        self(event);
    }
}

/// The set of [`BundleSubscriber`]s that a bundler reports to.
///
/// Every subscriber receives every event.
#[derive(Clone, Default)]
pub struct BundleEvents(Vec<Arc<dyn BundleSubscriber>>);

impl BundleEvents {
    pub(super) fn push(&mut self, subscriber: impl BundleSubscriber) {
        self.0.push(Arc::new(subscriber));
    }

    pub(super) fn emit(&self, event: &BundleEvent) {
        for subscriber in &self.0 {
            subscriber.handle(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    /// A subscriber that records the events it is handed.
    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<String>>>);

    impl Recorder {
        fn recorded(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }
    }

    impl BundleSubscriber for Recorder {
        fn handle(&self, event: &BundleEvent) {
            self.0.lock().unwrap().push(event.to_string());
        }
    }

    #[test]
    fn every_subscriber_sees_every_event() {
        let first = Recorder::default();
        let second = Recorder::default();

        let mut events = BundleEvents::default();
        events.push(first.clone());
        events.push(second.clone());

        events.emit(&BundleEvent::Scanned { count: 2 });
        events.emit(&BundleEvent::Removed {
            file: "stale.css".to_owned(),
        });

        assert_eq!(first.recorded(), ["found 2 assets", "removed stale.css"]);
        assert_eq!(first.recorded(), second.recorded());
    }

    #[test]
    fn a_closure_is_a_subscriber() {
        let recorder = Recorder::default();

        let mut events = BundleEvents::default();
        let sink = recorder.clone();
        events.push(move |event: &BundleEvent| sink.handle(event));
        events.emit(&BundleEvent::Scanned { count: 1 });

        assert_eq!(recorder.recorded(), ["found 1 assets"]);
    }

    #[test]
    fn emitting_without_subscribers_is_a_no_op() {
        BundleEvents::default().emit(&BundleEvent::Scanned { count: 0 });
    }

    #[test]
    fn events_describe_themselves() {
        let path = PathBuf::from("/cache/abc.css");
        let uri = Uri::from_static("https://example.com/app.css");

        assert_eq!(
            BundleEvent::CacheHit {
                uri: uri.clone(),
                path: path.clone(),
            }
            .to_string(),
            "cached https://example.com/app.css"
        );
        assert_eq!(
            BundleEvent::Downloaded {
                uri,
                path,
                bytes: 1024,
            }
            .to_string(),
            "downloaded https://example.com/app.css (1024 bytes)"
        );
        assert_eq!(
            BundleEvent::Bundled {
                id: AssetId::new("test", "src/lib.rs", "app.css", &crate::AssetOptions::NONE),
                file: "app-0123456789abcdef.css".to_owned(),
                bytes: 12,
            }
            .to_string(),
            "bundled app-0123456789abcdef.css (12 bytes)"
        );
        assert_eq!(
            BundleEvent::Finished {
                bundled: 3,
                unchanged: 4,
                removed: 1,
            }
            .to_string(),
            "bundled 3 assets (4 unchanged, 1 removed)"
        );
    }
}
