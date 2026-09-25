use std::path::PathBuf;

use console::style;
use tokio::task::JoinHandle;

use super::spinner::Spinner;
use crate::common::cargo::{BuildError, BuildOpts};

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

/// A background task that compiles the app and bundles its assets.
///
/// Reports progress and failures to the terminal. Dropping the handle cancels the task.
/// Prefer [`Self::cancel`] when the caller must wait for the task to release the
/// terminal.
pub struct BuildTask {
    handle: JoinHandle<Option<PathBuf>>,
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
    /// Returns the path of the built executable, or `None` when the build
    /// failed (the failure has already been reported to the terminal).
    ///
    /// Cancel-safe: if cancelled, the build keeps running and its result is
    /// returned by the next call.
    pub async fn finished(&mut self) -> Option<PathBuf> {
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
/// Returns the path of the built executable, or `None` after reporting the
/// failure to the terminal.
async fn build(kind: BuildKind, opts: BuildOpts) -> Option<PathBuf> {
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

    if let Err(error) = bundled {
        eprintln!(
            "  {}",
            style(format!("failed to bundle assets: {error}"))
                .red()
                .bold()
        );
        eprintln!();
        return None;
    }

    Some(exe)
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
