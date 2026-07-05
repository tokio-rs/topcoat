use std::path::PathBuf;

use console::style;
use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

/// How long a burst of filesystem events must stay quiet before it is
/// reported as a single change. Editors typically emit several events per
/// save, and operations like a branch switch touch many files at once.
const DEBOUNCE: Duration = Duration::from_millis(50);

/// Watches every workspace package's `src/` directory and coalesces bursts
/// of filesystem events into single change notifications.
pub struct SourceWatcher {
    /// Kept alive for its side effect: dropping the watcher stops the
    /// notifications feeding `events`.
    _watcher: notify::RecommendedWatcher,
    events: mpsc::UnboundedReceiver<()>,
    /// Set when an event has been consumed from `events` but not yet
    /// reported, because [`Self::changed`] was cancelled mid-debounce.
    pending: bool,
}

impl SourceWatcher {
    /// Start watching the source directories of the current workspace.
    pub async fn start() -> Self {
        let (tx, events) = mpsc::unbounded_channel();
        let mut watcher = recommended_watcher(move |event: notify::Result<notify::Event>| {
            // Access events fire on mere reads (an editor or tool scanning
            // the tree) and never indicate a source change.
            if let Ok(event) = &event
                && !matches!(event.kind, EventKind::Access(_))
            {
                let _ = tx.send(());
            }
        })
        .expect("failed to create file watcher");

        for dir in watch_dirs().await {
            watcher
                .watch(&dir, RecursiveMode::Recursive)
                .unwrap_or_else(|error| {
                    eprintln!(
                        "  {}",
                        style(format!("failed to watch {}: {error}", dir.display())).yellow()
                    );
                });
        }

        Self {
            _watcher: watcher,
            events,
            pending: false,
        }
    }

    /// Wait until a source file changes.
    ///
    /// A burst of events (a save producing several events, a branch switch
    /// touching many files) is reported as a single change: the call returns
    /// once the burst has been quiet for [`DEBOUNCE`].
    ///
    /// Cancel-safe: a change observed before cancellation is remembered and
    /// reported by the next call.
    pub async fn changed(&mut self) {
        if !self.pending {
            // The sender lives in the watcher's callback, so the channel
            // cannot close while `self` exists.
            self.events.recv().await.expect("watcher channel closed");
            self.pending = true;
        }
        while let Ok(Some(())) = timeout(DEBOUNCE, self.events.recv()).await {}
        self.pending = false;
    }
}

/// The `src/` directory of every package in the workspace, or `./src` when
/// `cargo metadata` is unavailable.
async fn watch_dirs() -> Vec<PathBuf> {
    let Some(metadata) = crate::cargo::metadata().await else {
        eprintln!(
            "  {}",
            style("cargo metadata failed; watching ./src").yellow()
        );
        return vec![PathBuf::from("./src")];
    };

    let dirs: Vec<PathBuf> = metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|package| {
            let manifest = PathBuf::from(package["manifest_path"].as_str()?);
            let src = manifest.parent()?.join("src");
            src.is_dir().then_some(src)
        })
        .collect();

    if dirs.is_empty() {
        vec![PathBuf::from("./src")]
    } else {
        dirs
    }
}
