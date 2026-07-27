use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::SystemTime;

use clap::Args;
use console::style;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use super::messages::Messages;
use super::progress::ProgressScanner;
use super::stderr::StderrTail;

/// Which target `cargo build` compiles and with which profile.
#[derive(Clone, Default)]
pub struct BuildOpts {
    pub bin: Option<String>,
    pub package: Option<String>,
    /// The cargo profile to build with, or `None` for the default (`dev`).
    pub profile: Option<String>,
}

impl BuildOpts {
    /// Compile the application and return the path of the final linked
    /// output, reporting cargo's `current/total` build progress to
    /// `on_progress` along the way.
    ///
    /// The bundler scans any linked binary for embedded asset declarations,
    /// so an executable and a cdylib/dylib (e.g. a `wasm32` build, which
    /// produces no executable) are equally valid outputs.
    pub async fn build(
        &self,
        mut on_progress: impl FnMut(u64, u64) + Send + 'static,
    ) -> Result<PathBuf, BuildError> {
        let mut child = self
            .command()
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(BuildError::Spawn)?;

        let mut stdout = child.stdout.take().expect("stdout piped");
        let mut stderr = child.stderr.take().expect("stderr piped");

        let stderr_task = tokio::spawn(async move {
            let mut captured = StderrTail::new();
            let mut progress = ProgressScanner::new();
            let mut buf = [0u8; 1024];
            loop {
                match stderr.read(&mut buf).await {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        captured.push(&buf[..n]);
                        if let Some((current, total)) = progress.push(&buf[..n]) {
                            on_progress(current, total);
                        }
                    }
                }
            }
            captured
        });

        let stdout_task = tokio::spawn(async move {
            let mut out = Vec::new();
            let _ = stdout.read_to_end(&mut out).await;
            out
        });

        let status = child.wait().await.map_err(BuildError::Wait)?;
        let stdout_bytes = stdout_task.await.unwrap_or_default();
        let stderr = stderr_task.await.unwrap_or_default();

        let messages = Messages::parse(&String::from_utf8_lossy(&stdout_bytes));

        if !status.success() {
            return Err(BuildError::Failed {
                diagnostics: messages.failure_diagnostics(&stderr),
            });
        }

        let mut artifacts = messages.artifacts();
        match artifacts.len() {
            0 => Err(BuildError::NoArtifact),
            1 => Ok(artifacts.remove(0)),
            _ => Err(BuildError::Multiple(artifacts)),
        }
    }

    /// [`Self::build`], plus the contents of the produced file.
    pub async fn build_and_read(
        &self,
        on_progress: impl FnMut(u64, u64) + Send + 'static,
    ) -> Result<(PathBuf, Vec<u8>), BuildError> {
        let path = self.build(on_progress).await?;
        let bytes = std::fs::read(&path).map_err(BuildError::Read)?;
        Ok((path, bytes))
    }

    /// The `cargo build` invocation for these options.
    fn command(&self) -> Command {
        let mut cmd = Command::new("cargo");
        // Strip env vars inherited from the outer `cargo run` that invoked
        // us, so the inner build has the same fingerprint as a plain `cargo
        // build` the user would run by hand. Otherwise CARGO/RUSTC/
        // RUSTC_WRAPPER/etc. shift the rustc/profile fingerprint hashes and
        // force cache-busting rebuilds.
        for (k, _) in std::env::vars_os() {
            let key = k.to_string_lossy();
            if key.starts_with("CARGO")
                || key == "RUSTC"
                || key == "RUSTC_WRAPPER"
                || key == "RUSTC_WORKSPACE_WRAPPER"
                || key == "RUSTUP_TOOLCHAIN"
                || key == "RUSTFLAGS"
            {
                cmd.env_remove(&k);
            }
        }
        cmd.args(["build", "--message-format=json-diagnostic-rendered-ansi"]);
        cmd.env("CARGO_TERM_PROGRESS_WHEN", "always");
        cmd.env("CARGO_TERM_PROGRESS_WIDTH", "80");
        if let Some(bin) = &self.bin {
            cmd.args(["--bin", bin]);
        }
        if let Some(package) = &self.package {
            cmd.args(["--package", package]);
        }
        // `--profile <name>` covers every profile: `--profile release` is
        // exactly equivalent to `--release`, and `--profile dev` to passing
        // nothing.
        if let Some(profile) = &self.profile {
            cmd.args(["--profile", profile]);
        }
        cmd
    }
}

/// Command-line flags selecting which target to build and with which profile.
///
/// Shared by every command that compiles the application, flattened into their
/// argument structs with `#[command(flatten)]` and converted into [`BuildOpts`]
/// with [`From::from`].
#[derive(Args)]
pub struct BuildFlags {
    /// Build the named binary target
    #[arg(long)]
    pub bin: Option<String>,
    /// Build the named package
    #[arg(short, long)]
    pub package: Option<String>,
    /// Build with the `release` profile
    #[arg(short, long, conflicts_with = "profile")]
    pub release: bool,
    /// Build with the named cargo profile
    #[arg(long, value_name = "NAME")]
    pub profile: Option<String>,
}

impl From<BuildFlags> for BuildOpts {
    fn from(flags: BuildFlags) -> Self {
        // `--release` is shorthand for `--profile release`; the two are
        // mutually exclusive, so at most one of these is set.
        let profile = flags
            .profile
            .or_else(|| flags.release.then(|| "release".to_string()));
        Self {
            bin: flags.bin,
            package: flags.package,
            profile,
        }
    }
}

pub enum BuildError {
    Spawn(io::Error),
    Wait(io::Error),
    /// Cargo reported a failed build. `diagnostics` is the error output to
    /// show, from [`Messages::failure_diagnostics`].
    Failed {
        diagnostics: String,
    },
    NoArtifact,
    Multiple(Vec<PathBuf>),
    Read(io::Error),
}

impl BuildError {
    pub fn print_and_exit(self) -> ! {
        eprintln!("{}", style(self.to_string()).red().bold());
        if let Self::Failed { diagnostics } = &self
            && !diagnostics.is_empty()
        {
            eprintln!();
            eprintln!("{diagnostics}");
        }
        std::process::exit(1);
    }
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "failed to spawn cargo build: {e}"),
            Self::Wait(e) => write!(f, "failed to wait for cargo build: {e}"),
            Self::Failed { .. } => write!(f, "build failed"),
            Self::NoArtifact => write!(
                f,
                "cargo build produced no executable or library to bundle; the crate must be a `bin`, `cdylib`, or `dylib` target"
            ),
            Self::Multiple(paths) => {
                write!(
                    f,
                    "cargo produced multiple targets; pass --bin or --package to choose one:"
                )?;
                for p in paths {
                    write!(f, "\n  {}", p.display())?;
                }
                Ok(())
            }
            Self::Read(e) => write!(f, "failed to read executable: {e}"),
        }
    }
}

impl fmt::Debug for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl std::error::Error for BuildError {}

/// Identity of a built executable: enough to tell whether a rebuild actually
/// produced a new binary. Cargo leaves the executable untouched when nothing
/// needed relinking, so an unchanged stamp means an unchanged application.
#[derive(PartialEq)]
pub struct BuildStamp {
    path: PathBuf,
    modified: SystemTime,
    len: u64,
}

impl BuildStamp {
    /// Read the stamp of the executable at `path`, or `None` when it cannot
    /// be inspected.
    pub fn of(path: &Path) -> Option<Self> {
        let metadata = std::fs::metadata(path).ok()?;
        Some(Self {
            path: path.to_path_buf(),
            modified: metadata.modified().ok()?,
            len: metadata.len(),
        })
    }
}
