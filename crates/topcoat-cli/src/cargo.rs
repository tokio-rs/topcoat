use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use clap::Args;
use console::style;
use tokio::io::AsyncReadExt;
use tokio::process::Command;

/// Run `cargo metadata --no-deps` for the current workspace and parse its
/// output. Returns `None` when cargo cannot be spawned or reports an error.
pub async fn metadata() -> Option<serde_json::Value> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version=1"])
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

pub async fn build(
    opts: &BuildOpts,
    mut on_progress: impl FnMut(u64, u64) + Send + 'static,
) -> Result<PathBuf, BuildError> {
    let mut cmd = Command::new("cargo");
    // Strip env vars inherited from the outer `cargo run` that invoked us, so
    // the inner build has the same fingerprint as a plain `cargo build` the
    // user would run by hand. Otherwise CARGO/RUSTC/RUSTC_WRAPPER/etc. shift
    // the rustc/profile fingerprint hashes and force cache-busting rebuilds.
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
