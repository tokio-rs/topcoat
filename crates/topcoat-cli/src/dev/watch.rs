use std::collections::{BTreeSet, HashSet};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use console::style;
use ignore::WalkBuilder;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};
use tokio::sync::mpsc;
use tokio::time::{Duration, timeout};

/// How long a burst of filesystem events must stay quiet before it is
/// reported as a single change. Editors typically emit several events per
/// save, and operations like a branch switch touch many files at once.
const DEBOUNCE: Duration = Duration::from_millis(50);

/// A change reported by [`SourceWatcher::changed`].
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Change {
    /// A watched file changed; a rebuild is due.
    Source,
    /// A `Cargo.toml` changed. The dependency graph may have gained or lost
    /// local packages, so the watcher should be recreated to pick up the new
    /// set of directories before rebuilding.
    Manifest,
}

/// Watches every local package in the dependency graph and coalesces bursts
/// of filesystem events into single change notifications.
///
/// "Local" covers the workspace members and every path dependency they pull
/// in, wherever it lives on disk. Within each package directory everything
/// except gitignored paths, hidden entries, editor temp files, and the cargo
/// target directory is watched: manifests, build scripts, and files embedded
/// with `asset!` or `include_str!` trigger rebuilds just like Rust sources.
pub struct SourceWatcher {
    watcher: notify::RecommendedWatcher,
    events: mpsc::UnboundedReceiver<Change>,
    /// Set when a change has been consumed from `events` but not yet
    /// reported, because [`Self::changed`] was cancelled mid-debounce.
    pending: Option<Change>,
    roots: Vec<PathBuf>,
    /// Subdirectories of the roots currently under a recursive watch.
    watched: HashSet<PathBuf>,
    filter: Arc<PathFilter>,
}

impl SourceWatcher {
    /// Start watching the local packages of the current workspace.
    pub async fn start() -> Self {
        let plan = WatchPlan::discover().await;
        let filter = Arc::new(plan.filter);

        let (tx, events) = mpsc::unbounded_channel();
        let callback_filter = Arc::clone(&filter);
        let mut watcher = recommended_watcher(move |event: notify::Result<notify::Event>| {
            let Ok(event) = event else { return };
            // Access events fire on mere reads (an editor or tool scanning
            // the tree) and never indicate a source change.
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            // A rescan means events were dropped; assume something changed.
            let mut change = event.need_rescan().then_some(Change::Source);
            for path in &event.paths {
                if callback_filter.ignores(path) {
                    continue;
                }
                if path.file_name().is_some_and(|name| name == "Cargo.toml") {
                    change = Some(Change::Manifest);
                    break;
                }
                change = Some(Change::Source);
            }
            if let Some(change) = change {
                let _ = tx.send(change);
            }
        })
        .expect("failed to create file watcher");

        // The roots themselves are watched non-recursively: that covers
        // manifests and other files at the top level, while subdirectories
        // get dedicated recursive watches in `resync`. This keeps the
        // (potentially huge) target directory from ever being registered
        // with the OS.
        for root in &plan.roots {
            if let Err(error) = watcher.watch(root, RecursiveMode::NonRecursive) {
                report_watch_error(root, &error);
            }
        }

        let mut watcher = Self {
            watcher,
            events,
            pending: None,
            roots: plan.roots,
            watched: HashSet::new(),
            filter,
        };
        watcher.resync();
        watcher
    }

    /// Wait until a watched file changes.
    ///
    /// A burst of events (a save producing several events, a branch switch
    /// touching many files) is reported as a single change: the call returns
    /// once the burst has been quiet for [`DEBOUNCE`]. A [`Change::Manifest`]
    /// anywhere in the burst takes precedence over plain source changes.
    ///
    /// Cancel-safe: a change observed before cancellation is remembered and
    /// reported by the next call.
    pub async fn changed(&mut self) -> Change {
        if self.pending.is_none() {
            // The sender lives in the watcher's callback, so the channel
            // cannot close while `self` exists.
            let change = self.events.recv().await.expect("watcher channel closed");
            self.pending = Some(change);
        }
        while let Ok(Some(change)) = timeout(DEBOUNCE, self.events.recv()).await {
            if change == Change::Manifest {
                self.pending = Some(Change::Manifest);
            }
        }
        // A directory created during the burst needs a watch of its own.
        self.resync();
        self.pending.take().expect("pending change set above")
    }

    /// Bring the recursive watches in line with the subdirectories currently
    /// present under the roots, picking up any created since the last sync.
    fn resync(&mut self) {
        self.watched.retain(|dir| dir.is_dir());
        let mut fresh = Vec::new();
        for root in &self.roots {
            let Ok(entries) = std::fs::read_dir(root) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && !self.watched.contains(&path) && !self.filter.ignores(&path) {
                    fresh.push(path);
                }
            }
        }
        for dir in fresh {
            if let Err(error) = self.watcher.watch(&dir, RecursiveMode::Recursive) {
                report_watch_error(&dir, &error);
            }
            // Recorded even on failure, so the error is not repeated on
            // every subsequent change.
            self.watched.insert(dir);
        }
    }
}

/// The directories to watch and the filter deciding which events within
/// them matter.
struct WatchPlan {
    /// The manifest directory of every local package plus the workspace
    /// root, minus roots contained in another root.
    roots: Vec<PathBuf>,
    filter: PathFilter,
}

impl WatchPlan {
    /// Derive the plan for the current workspace from cargo metadata, or
    /// fall back to watching `./src` when metadata is unavailable.
    async fn discover() -> Self {
        let Some(metadata) = crate::cargo::full_metadata().await else {
            eprintln!(
                "  {}",
                style("cargo metadata failed; watching ./src").yellow()
            );
            let roots = vec![canonical(PathBuf::from("./src"))];
            return Self {
                filter: PathFilter {
                    roots: roots.clone(),
                    target_dir: None,
                    matchers: Vec::new(),
                },
                roots,
            };
        };

        // A package without a `source` is local: a workspace member or a
        // path dependency, wherever it lives on disk.
        let mut roots: Vec<PathBuf> = metadata["packages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|package| package["source"].is_null())
            .filter_map(|package| {
                let manifest = Path::new(package["manifest_path"].as_str()?);
                Some(canonical(manifest.parent()?.to_path_buf()))
            })
            .filter(|dir| dir.is_dir())
            .collect();
        // The workspace root holds the root manifest and lockfile, and in a
        // virtual workspace it is not a package of its own.
        if let Some(root) = metadata["workspace_root"].as_str() {
            roots.push(canonical(PathBuf::from(root)));
        }
        let roots = dedupe_roots(roots);

        let target_dir = metadata["target_directory"]
            .as_str()
            .map(|dir| canonical(PathBuf::from(dir)));
        let matchers = gitignore_matchers(&roots, target_dir.as_deref());

        Self {
            filter: PathFilter {
                roots: roots.clone(),
                target_dir,
                matchers,
            },
            roots,
        }
    }
}

/// Decides which filesystem paths are relevant to a rebuild.
struct PathFilter {
    /// The watched roots; the hidden-entry check applies only to path
    /// components below a root, since the root's own path may legitimately
    /// contain hidden components.
    roots: Vec<PathBuf>,
    /// The cargo target directory. Cargo writes here throughout a build, so
    /// reacting to it would have the watcher feeding on the very rebuilds it
    /// triggers.
    target_dir: Option<PathBuf>,
    matchers: Vec<Gitignore>,
}

impl PathFilter {
    /// Whether an event for `path` should be discarded.
    fn ignores(&self, path: &Path) -> bool {
        if let Some(target) = &self.target_dir
            && path.starts_with(target)
        {
            return true;
        }

        if path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(is_editor_temp)
        {
            return true;
        }

        // Hidden entries, `.git` most prominently: a branch switch churns
        // through it far more than through the files it updates.
        if let Some(rel) = self
            .roots
            .iter()
            .find_map(|root| path.strip_prefix(root).ok())
            && rel
                .components()
                .any(|component| component.as_os_str().to_string_lossy().starts_with('.'))
        {
            return true;
        }

        let is_dir = path.is_dir();
        self.matchers.iter().any(|matcher| {
            // A matcher panics on paths outside its root, so scope it first.
            path.starts_with(matcher.path())
                && matcher
                    .matched_path_or_any_parents(path, is_dir)
                    .is_ignore()
        })
    }
}

/// Editor write patterns that gitignore files rarely cover: backup and
/// atomic-save artifacts dropped next to the file being edited.
fn is_editor_temp(name: &str) -> bool {
    name.ends_with('~') // backup copies (vim, emacs)
        || name == "4913" // vim's write-permission probe
        || (name.starts_with('#') && name.ends_with('#')) // emacs autosave
        || name.contains("___jb_") // jetbrains safe-write temps
}

/// Sort roots and drop any root contained in another, so no directory is
/// watched twice.
fn dedupe_roots(mut roots: Vec<PathBuf>) -> Vec<PathBuf> {
    roots.sort();
    roots.dedup();
    let mut kept: Vec<PathBuf> = Vec::new();
    for root in roots {
        if !kept.iter().any(|ancestor| root.starts_with(ancestor)) {
            kept.push(root);
        }
    }
    kept
}

/// Build a matcher for every gitignore-style file applying somewhere under
/// the roots: `.gitignore` files below each root, in it, and above it (up to
/// the repository root), plus each repository's `.git/info/exclude`.
fn gitignore_matchers(roots: &[PathBuf], target_dir: Option<&Path>) -> Vec<Gitignore> {
    let mut gitignores = BTreeSet::new();
    let mut excludes = BTreeSet::new();

    for root in roots {
        // `.gitignore` files above the root still apply to it: the
        // repository root of an external path dependency is often a few
        // directories up.
        for dir in root.ancestors() {
            let gitignore = dir.join(".gitignore");
            if gitignore.is_file() {
                gitignores.insert(gitignore);
            }
            if dir.join(".git").exists() {
                let exclude = dir.join(".git/info/exclude");
                if exclude.is_file() {
                    excludes.insert((dir.to_path_buf(), exclude));
                }
                break;
            }
        }

        // `.gitignore` files nested below the root. The walk itself honors
        // gitignore rules, so ignored subtrees are not descended into.
        let target = target_dir.map(Path::to_path_buf);
        let mut walk = WalkBuilder::new(root);
        walk.require_git(false)
            .filter_entry(move |entry| Some(entry.path()) != target.as_deref());
        for entry in walk.build().flatten() {
            if entry.file_type().is_some_and(|kind| kind.is_dir()) {
                let gitignore = entry.path().join(".gitignore");
                if gitignore.is_file() {
                    gitignores.insert(gitignore);
                }
            }
        }
    }

    let mut matchers = Vec::new();
    for path in gitignores {
        // Rooted at the directory containing the file, as git scopes it.
        let (matcher, _error) = Gitignore::new(&path);
        if !matcher.is_empty() {
            matchers.push(matcher);
        }
    }
    for (repo_root, exclude) in excludes {
        // Exclude patterns are relative to the repository root, not to the
        // `.git/info` directory holding the file.
        let mut builder = GitignoreBuilder::new(&repo_root);
        builder.add(&exclude);
        if let Ok(matcher) = builder.build()
            && !matcher.is_empty()
        {
            matchers.push(matcher);
        }
    }
    matchers
}

/// Canonicalize where possible, so paths compare equal to the resolved
/// paths the watcher reports events for.
fn canonical(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn report_watch_error(path: &Path, error: &notify::Error) {
    eprintln!(
        "  {}",
        style(format!("failed to watch {}: {error}", path.display())).yellow()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn editor_temp_names() {
        assert!(is_editor_temp("main.rs~"));
        assert!(is_editor_temp("4913"));
        assert!(is_editor_temp("#main.rs#"));
        assert!(is_editor_temp("main.rs.___jb_tmp___"));
        assert!(!is_editor_temp("main.rs"));
        assert!(!is_editor_temp("Cargo.toml"));
    }

    #[test]
    fn dedupe_drops_contained_roots() {
        let roots = vec![
            PathBuf::from("/work/app/crates/web"),
            PathBuf::from("/work/app"),
            PathBuf::from("/work/app"),
            PathBuf::from("/work/lib"),
        ];
        let expected = vec![PathBuf::from("/work/app"), PathBuf::from("/work/lib")];
        assert_eq!(dedupe_roots(roots), expected);
    }

    #[test]
    fn filter_ignores_irrelevant_paths() {
        let root = std::env::temp_dir().join(format!("topcoat-watch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".gitignore"), "generated/\n*.log\n").unwrap();

        let filter = PathFilter {
            roots: vec![root.clone()],
            target_dir: Some(root.join("target")),
            matchers: gitignore_matchers(std::slice::from_ref(&root), None),
        };

        assert!(filter.ignores(&root.join("target/debug/app")));
        assert!(filter.ignores(&root.join(".git/HEAD")));
        assert!(filter.ignores(&root.join("src/.main.rs.swp")));
        assert!(filter.ignores(&root.join("src/main.rs~")));
        assert!(filter.ignores(&root.join("generated/out.rs")));
        assert!(filter.ignores(&root.join("server.log")));
        assert!(!filter.ignores(&root.join("src/main.rs")));
        assert!(!filter.ignores(&root.join("Cargo.toml")));
        assert!(!filter.ignores(&root.join("assets/logo.svg")));

        std::fs::remove_dir_all(&root).unwrap();
    }
}
