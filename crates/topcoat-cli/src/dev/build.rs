use std::path::{Path, PathBuf};

use console::style;
use tokio::task::JoinHandle;
use topcoat_asset::MANIFEST_NAME;

use super::spinner::Spinner;
use crate::common::cargo::{BuildError, BuildOpts, BuildStamp as ExecutableStamp};

/// Which kind of build a [`BuildTask`] runs; affects only the progress label.
#[derive(Clone, Copy)]
pub enum BuildKind {
    /// The first build after `topcoat dev` starts.
    Initial,
    /// A build triggered by a source change.
    Rebuild,
}

impl BuildKind {
    fn label(self) -> &'static str {
        match self {
            Self::Initial => "building",
            Self::Rebuild => "rebuilding",
        }
    }
}

/// The executable and asset bundle produced by a successful build.
pub struct BuildOutput {
    pub exe: PathBuf,
    pub stamp: Option<BuildStamp>,
}

/// A snapshot used to compare a build with the running application's build.
#[derive(PartialEq)]
pub struct BuildStamp {
    executable: ExecutableStamp,
    manifest: Vec<u8>,
}

impl BuildStamp {
    /// Read both outputs, or return `None` if either cannot be inspected.
    fn of(exe: &Path, assets: &Path) -> Option<Self> {
        Some(Self {
            executable: ExecutableStamp::of(exe)?,
            // Bundling rewrites the manifest even when every asset is unchanged,
            // so compare its contents rather than its file metadata.
            manifest: std::fs::read(assets.join(MANIFEST_NAME)).ok()?,
        })
    }
}

/// A background task that compiles the app and bundles its assets.
///
/// Reports progress and failures to the terminal. Dropping the handle cancels the task.
/// Prefer [`Self::cancel`] when the caller must wait for the task to release the
/// terminal.
pub struct BuildTask {
    handle: JoinHandle<Option<BuildOutput>>,
}

impl BuildTask {
    /// Start a build in the background.
    pub fn spawn(kind: BuildKind, opts: BuildOpts) -> Self {
        Self {
            handle: tokio::spawn(build(kind, opts)),
        }
    }

    /// Wait for the build to finish.
    ///
    /// Returns the built outputs, or `None` when the build failed (the failure
    /// has already been reported to the terminal).
    ///
    /// Cancel-safe: if cancelled, the build keeps running and its result is
    /// returned by the next call.
    pub async fn finished(&mut self) -> Option<BuildOutput> {
        (&mut self.handle).await.expect("build task panicked")
    }

    /// Cancel the build, killing the underlying cargo process, and wait for
    /// the task to be torn down.
    pub async fn cancel(mut self) {
        self.handle.abort();
        let _ = (&mut self.handle).await;
    }
}

impl Drop for BuildTask {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

/// Compile the application and bundle its assets.
///
/// Returns the built outputs, or `None` after reporting the failure to the terminal.
async fn build(kind: BuildKind, opts: BuildOpts) -> Option<BuildOutput> {
    let label = kind.label();
    let spinner = Spinner::new(label);
    let progress = spinner.bar();
    let result = opts
        .build_and_read(move |current, total| {
            progress.set_message(format!("{label} ({current}/{total})"));
        })
        .await;
    drop(spinner);

    let (exe, bytes) = match result {
        Ok(built) => built,
        Err(error) => {
            report_build_error(&error);
            return None;
        }
    };

    let spinner = Spinner::new("bundling assets");
    let bundled = crate::asset::run_bundle(&exe, &bytes, None).await;
    drop(spinner);

    let assets = match bundled {
        Ok(assets) => assets,
        Err(error) => {
            eprintln!(
                "  {}",
                style(format!("failed to bundle assets: {error}"))
                    .red()
                    .bold()
            );
            eprintln!();
            return None;
        }
    };

    Some(BuildOutput {
        stamp: BuildStamp::of(&exe, &assets),
        exe,
    })
}

fn report_build_error(error: &BuildError) {
    eprintln!("  {}", style("build failed").red().bold());
    eprintln!();
    match error {
        BuildError::Failed { diagnostics } => {
            if !diagnostics.is_empty() {
                eprintln!("{diagnostics}");
            }
        }
        _ => eprintln!("  {}", style(error.to_string()).red()),
    }
    eprintln!();
}

#[cfg(test)]
mod tests {
    use std::{fs, time::UNIX_EPOCH};

    use topcoat_asset::{AssetId, AssetOptions, Bundler, BundlerConfig, RawAsset};

    use super::*;

    struct Fixture {
        root: PathBuf,
        exe: PathBuf,
        assets: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("topcoat-dev-build-{name}-{}", std::process::id()));
            fs::create_dir_all(&root).unwrap();
            let source = root.join("logo.txt");
            fs::write(&source, "before").unwrap();
            let path = source.to_str().unwrap();
            let id = AssetId::new("test", "src/main.rs", path, &AssetOptions::NONE);
            let binary = RawAsset::encode(
                id,
                path,
                "test",
                root.to_str().unwrap(),
                "src/main.rs",
                &AssetOptions::NONE,
            );
            let exe = root.join("app");
            fs::write(&exe, binary).unwrap();
            Self {
                exe,
                assets: root.join("assets"),
                root,
            }
        }

        fn bundle(&self) {
            let config = BundlerConfig::new().cache_dir(self.root.join("cache"));
            Bundler::new(&config)
                .bundle(&fs::read(&self.exe).unwrap(), &self.assets)
                .unwrap();
        }

        fn stamp(&self) -> BuildStamp {
            BuildStamp::of(&self.exe, &self.assets).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn asset_only_change_differs_from_running_build_after_repeated_bundles() {
        let fixture = Fixture::new("asset-change");
        fixture.bundle();
        let running = fixture.stamp();

        fs::write(fixture.root.join("logo.txt"), "after!").unwrap();
        fixture.bundle();
        let changed = fixture.stamp();
        assert!(changed.executable == running.executable);
        assert!(changed != running);

        // A completed bundle can be discarded when another rebuild is queued.
        // Even if the next bundle writes nothing new, the app must restart.
        fixture.bundle();
        let next = fixture.stamp();
        assert!(next == changed);
        assert!(next != running);
    }

    #[test]
    fn unchanged_assets_keep_stamp_when_manifest_is_rewritten() {
        let fixture = Fixture::new("unchanged");
        fixture.bundle();
        let manifest = fixture.assets.join(MANIFEST_NAME);
        fs::File::options()
            .write(true)
            .open(&manifest)
            .unwrap()
            .set_modified(UNIX_EPOCH)
            .unwrap();
        let running = fixture.stamp();

        fixture.bundle();
        assert_ne!(
            fs::metadata(manifest).unwrap().modified().unwrap(),
            UNIX_EPOCH
        );
        assert!(fixture.stamp() == running);
    }

    #[test]
    fn missing_outputs_cannot_be_stamped() {
        let fixture = Fixture::new("missing");
        fixture.bundle();

        fs::remove_file(fixture.assets.join(MANIFEST_NAME)).unwrap();
        assert!(BuildStamp::of(&fixture.exe, &fixture.assets).is_none());

        fixture.bundle();
        fs::remove_file(&fixture.exe).unwrap();
        assert!(BuildStamp::of(&fixture.exe, &fixture.assets).is_none());
    }
}
