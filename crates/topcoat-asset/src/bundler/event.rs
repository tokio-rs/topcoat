use std::{fmt, path::PathBuf, sync::Arc};

use http::Uri;

use crate::Asset;

/// A step the [`Bundler`](super::Bundler) took while syncing assets.
///
/// Events are reported as they happen, from whichever worker thread did
/// the work, so they can arrive in any order relative to one another.
/// [`Scanned`](Self::Scanned) is always first and
/// [`Finished`](Self::Finished) always last.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum BundleEvent {
    /// Scanning the binary turned up `count` asset declarations.
    Scanned { count: usize },
    /// A remote asset was already in the download cache.
    CacheHit { uri: Uri, path: PathBuf },
    /// A remote asset is not cached and is about to be downloaded.
    DownloadStarted { uri: Uri },
    /// A remote asset finished downloading into the cache.
    Downloaded { uri: Uri, path: PathBuf, bytes: u64 },
    /// An asset was written into the bundle directory.
    Bundled { id: Asset, file: String, bytes: usize },
    /// An asset was already in the bundle directory with the same
    /// contents, so it was left alone.
    Unchanged { id: Asset, file: String },
    /// A file that is no longer referenced was deleted from the bundle
    /// directory.
    Removed { file: String },
    /// Every asset has been processed and the manifest is written.
    Finished {
        bundled: usize,
        unchanged: usize,
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

/// A consumer of [`BundleEvent`]s, registered with
/// [`BundlerConfig::subscribe`](super::BundlerConfig::subscribe).
///
/// Implemented for any `Fn(&BundleEvent) + Send + Sync + 'static`, so a
/// closure is usually all you need. Handlers run inline on the bundler's
/// worker threads: keep them cheap, and hand off to a channel or a
/// background thread for anything slow.
pub trait BundleSubscriber: Send + Sync + 'static {
    /// Handle a single event.
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

/// The set of [`BundleSubscriber`]s a bundler reports to.
///
/// Every subscriber sees every event, so a bundle run can drive a
/// progress bar, a log, and a channel at the same time.
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
