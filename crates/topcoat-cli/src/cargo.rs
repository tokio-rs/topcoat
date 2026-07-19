use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::SystemTime;

use clap::Args;
use console::style;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Run `cargo metadata --no-deps` for the current workspace and parse its
/// output. Returns `None` when cargo cannot be spawned or reports an error.
pub async fn metadata() -> Option<serde_json::Value> {
    run_metadata(&["--no-deps"]).await
}

/// Run `cargo metadata` with the dependency graph resolved, so the output
/// also lists path dependencies living outside the workspace. Returns `None`
/// when cargo cannot be spawned or reports an error.
pub async fn full_metadata() -> Option<serde_json::Value> {
    run_metadata(&[]).await
}

async fn run_metadata(extra_args: &[&str]) -> Option<serde_json::Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version=1"])
        .args(extra_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .ok()?
        .wait_with_output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    serde_json::from_slice(&output.stdout).ok()
}

/// The workspace's cargo target directory, from [`metadata`].
pub async fn target_dir() -> Option<PathBuf> {
    let metadata = metadata().await?;
    metadata
        .get("target_directory")?
        .as_str()
        .map(PathBuf::from)
}

#[derive(Clone, Default)]
pub struct BuildOpts {
    pub bin: Option<String>,
    pub package: Option<String>,
    /// The cargo profile to build with, or `None` for the default (`dev`).
    pub profile: Option<String>,
}

/// Command-line flags selecting which target to build and with which profile.
///
/// Shared by every command that compiles the application, flattened into their
/// argument structs with `#[command(flatten)]` and converted into [`BuildOpts`]
/// with [`Into::into`].
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
    Failed { rendered: String },
    NoExecutable,
    Multiple(Vec<PathBuf>),
    Read(io::Error),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "failed to spawn cargo build: {e}"),
            Self::Failed { .. } => write!(f, "build failed"),
            Self::NoExecutable => write!(f, "no executable produced by cargo build"),
            Self::Multiple(paths) => {
                write!(
                    f,
                    "cargo produced multiple binaries; pass --bin or --package to choose one:"
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

impl BuildError {
    pub fn print_and_exit(self) -> ! {
        eprintln!("{}", style(self.to_string()).red().bold());
        if let Self::Failed { rendered } = &self
            && !rendered.is_empty()
        {
            eprintln!();
            eprint!("{rendered}");
        }
        std::process::exit(1);
    }
}

/// Whether `key` is stripped from the environment of the inner `cargo build`.
///
/// The goal is for the inner build to have the same fingerprint as a plain
/// `cargo build` the user would run by hand. When the CLI itself runs under
/// `cargo run`, the outer cargo injects variables (`CARGO`,
/// `CARGO_MANIFEST_DIR`, `CARGO_PKG_*`, `RUSTUP_TOOLCHAIN`, ...) that a
/// hand-run build would not see; left in place they shift the rustc/profile
/// fingerprint hashes and force cache-busting rebuilds. Those injected
/// variables are stripped by name.
///
/// User configuration must survive the sweep: a hand-run build in an
/// environment with a non-default `CARGO_HOME` (e.g. the official rust Docker
/// images set `CARGO_HOME=/usr/local/cargo`) or `CARGO_TARGET_DIR`,
/// `CARGO_NET_*`, `CARGO_REGISTRIES_*`, etc. sees those variables too, so
/// stripping them would defeat the goal -- losing `CARGO_HOME` in particular
/// silently sends the inner build to `~/.cargo`, re-downloading the registry
/// and recompiling every dependency from scratch.
fn should_strip_env(key: &str) -> bool {
    // The variables cargo sets for child processes, per "Environment
    // variables Cargo sets for crates" in the cargo book. `CARGO_HOME` is
    // also on that list but is deliberately kept: cargo injects it with the
    // same value a hand-run build resolves, and the inner build needs it to
    // find the registry cache.
    matches!(
        key,
        "CARGO"
            | "CARGO_MANIFEST_DIR"
            | "CARGO_MANIFEST_PATH"
            | "CARGO_CRATE_NAME"
            | "CARGO_BIN_NAME"
            | "CARGO_PRIMARY_PACKAGE"
            | "CARGO_TARGET_TMPDIR"
            | "CARGO_MAKEFLAGS"
            | "RUSTC"
            | "RUSTC_WRAPPER"
            | "RUSTC_WORKSPACE_WRAPPER"
            | "RUSTUP_TOOLCHAIN"
            | "RUSTFLAGS"
    ) || key.starts_with("CARGO_PKG_")
        || key.starts_with("CARGO_BIN_EXE_")
}

pub async fn build(
    opts: &BuildOpts,
    mut on_progress: impl FnMut(u64, u64) + Send + 'static,
) -> Result<PathBuf, BuildError> {
    let mut cmd = Command::new("cargo");
    for (k, _) in std::env::vars_os() {
        if should_strip_env(&k.to_string_lossy()) {
            cmd.env_remove(&k);
        }
    }
    cmd.args(["build", "--message-format=json-diagnostic-rendered-ansi"]);
    cmd.env("CARGO_TERM_PROGRESS_WHEN", "always");
    cmd.env("CARGO_TERM_PROGRESS_WIDTH", "80");
    if let Some(bin) = &opts.bin {
        cmd.args(["--bin", bin]);
    }
    if let Some(package) = &opts.package {
        cmd.args(["--package", package]);
    }
    // `--profile <name>` covers every profile: `--profile release` is exactly
    // equivalent to `--release`, and `--profile dev` to passing nothing.
    if let Some(profile) = &opts.profile {
        cmd.args(["--profile", profile]);
    }

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(BuildError::Spawn)?;

    let mut stdout = child.stdout.take().expect("stdout piped");
    let mut stderr = child.stderr.take().expect("stderr piped");

    let stderr_task = tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        let mut tail: Vec<u8> = Vec::with_capacity(512);
        let mut last_emitted: Option<(u64, u64)> = None;
        loop {
            match stderr.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    tail.extend_from_slice(&buf[..n]);
                    if let Some(prog) = scan_last_progress(&tail)
                        && last_emitted != Some(prog)
                    {
                        last_emitted = Some(prog);
                        on_progress(prog.0, prog.1);
                    }
                    if tail.len() > 512 {
                        let drain_to = tail.len() - 128;
                        tail.drain(..drain_to);
                    }
                }
            }
        }
    });

    let stdout_task = tokio::spawn(async move {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out).await;
        out
    });

    let status = child.wait().await.map_err(BuildError::Spawn)?;
    let stdout_bytes = stdout_task.await.unwrap_or_default();
    let _ = stderr_task.await;

    let stdout_str = String::from_utf8_lossy(&stdout_bytes);
    let messages: Vec<serde_json::Value> = stdout_str
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if !status.success() {
        let rendered: String = messages
            .iter()
            .filter_map(|msg| {
                msg.get("message")
                    .and_then(|m| m.get("rendered"))
                    .and_then(|r| r.as_str())
            })
            .collect();
        return Err(BuildError::Failed { rendered });
    }

    let executables: Vec<PathBuf> = messages
        .iter()
        .filter_map(|msg| {
            if msg.get("reason")?.as_str()? == "compiler-artifact" {
                msg.get("executable")?.as_str().map(PathBuf::from)
            } else {
                None
            }
        })
        .collect();

    match executables.len() {
        0 => Err(BuildError::NoExecutable),
        1 => Ok(executables.into_iter().next().unwrap()),
        _ => Err(BuildError::Multiple(executables)),
    }
}

pub async fn build_and_read(
    opts: &BuildOpts,
    on_progress: impl FnMut(u64, u64) + Send + 'static,
) -> Result<(PathBuf, Vec<u8>), BuildError> {
    let path = build(opts, on_progress).await?;
    let bytes = std::fs::read(&path).map_err(BuildError::Read)?;
    Ok((path, bytes))
}

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

fn scan_last_progress(bytes: &[u8]) -> Option<(u64, u64)> {
    let mut last = None;
    let mut i = 0;
    while i < bytes.len() {
        if !bytes[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let mid = i;
        if i >= bytes.len() || bytes[i] != b'/' {
            continue;
        }
        i += 1;
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            continue;
        }
        let t_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let cur: Option<u64> = std::str::from_utf8(&bytes[start..mid])
            .ok()
            .and_then(|s| s.parse().ok());
        let total: Option<u64> = std::str::from_utf8(&bytes[t_start..i])
            .ok()
            .and_then(|s| s.parse().ok());
        if let (Some(c), Some(t)) = (cur, total)
            && c <= t
            && t > 0
        {
            last = Some((c, t));
        }
    }
    last
}

#[cfg(test)]
mod tests {
    use super::should_strip_env;

    #[test]
    fn strips_cargo_injected_vars() {
        for key in [
            "CARGO",
            "CARGO_MANIFEST_DIR",
            "CARGO_MANIFEST_PATH",
            "CARGO_PKG_NAME",
            "CARGO_PKG_VERSION_MAJOR",
            "CARGO_BIN_EXE_topcoat",
            "CARGO_CRATE_NAME",
            "CARGO_PRIMARY_PACKAGE",
            "RUSTC",
            "RUSTC_WRAPPER",
            "RUSTC_WORKSPACE_WRAPPER",
            "RUSTUP_TOOLCHAIN",
            "RUSTFLAGS",
        ] {
            assert!(should_strip_env(key), "{key} should be stripped");
        }
    }

    #[test]
    fn keeps_user_configuration() {
        for key in [
            "CARGO_HOME",
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_JOBS",
            "CARGO_NET_OFFLINE",
            "CARGO_REGISTRIES_MY_REGISTRY_TOKEN",
            "CARGO_TERM_COLOR",
            "CARGO_INCREMENTAL",
            "CARGO_PROFILE_RELEASE_LTO",
            "RUSTUP_HOME",
            "PATH",
        ] {
            assert!(!should_strip_env(key), "{key} should be kept");
        }
    }
}
