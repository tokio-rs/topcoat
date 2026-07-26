use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::SystemTime;

use clap::Args;
use console::{strip_ansi_codes, style};
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
    Wait(io::Error),
    /// Cargo reported a failed build. `diagnostics` is the error output to
    /// show, from [`failure_diagnostics`].
    Failed {
        diagnostics: String,
    },
    NoArtifact,
    Multiple(Vec<PathBuf>),
    Read(io::Error),
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

/// How much of cargo's stderr to keep for reporting a failure.
///
/// Cargo redraws its progress bar in place, so most of a long build's stderr
/// is renders that were immediately overwritten. Keeping only the tail bounds
/// that without losing the error report, which cargo prints last.
const STDERR_CAPTURE_LIMIT: usize = 256 * 1024;

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
    // MSVC's linker strips the `#[used]` statics that carry the embedded
    // asset declarations (see `topcoat-asset`), so on Windows the bundler
    // finds no assets and every page panics resolving them. /OPT:NOREF keeps
    // unreferenced data alive. RUSTFLAGS was stripped above, so this sets a
    // deterministic value rather than appending to user state.
    if cfg!(all(windows, target_env = "msvc")) {
        cmd.env("RUSTFLAGS", "-C link-arg=/OPT:NOREF");
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
        let mut captured: Vec<u8> = Vec::new();
        let mut tail: Vec<u8> = Vec::with_capacity(512);
        let mut last_emitted: Option<(u64, u64)> = None;
        loop {
            match stderr.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    captured.extend_from_slice(&buf[..n]);
                    if captured.len() > STDERR_CAPTURE_LIMIT * 2 {
                        let drain_to = captured.len() - STDERR_CAPTURE_LIMIT;
                        captured.drain(..drain_to);
                    }
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
        captured
    });

    let stdout_task = tokio::spawn(async move {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out).await;
        out
    });

    let status = child.wait().await.map_err(BuildError::Wait)?;
    let stdout_bytes = stdout_task.await.unwrap_or_default();
    let stderr_bytes = stderr_task.await.unwrap_or_default();

    let stdout_str = String::from_utf8_lossy(&stdout_bytes);
    let messages: Vec<serde_json::Value> = stdout_str
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if !status.success() {
        let stderr_str = String::from_utf8_lossy(&stderr_bytes);
        return Err(BuildError::Failed {
            diagnostics: failure_diagnostics(&messages, &stderr_str),
        });
    }

    // The bundler scans any linked binary for embedded asset declarations, so
    // an executable and a cdylib/dylib (e.g. a `wasm32` build, which produces
    // no executable) are equally valid targets.
    let artifacts = select_artifacts(&messages);

    match artifacts.len() {
        0 => Err(BuildError::NoArtifact),
        1 => Ok(artifacts.into_iter().next().unwrap()),
        _ => Err(BuildError::Multiple(artifacts)),
    }
}

/// The error output to show for a build cargo reported as failed.
///
/// rustc reports its diagnostics as JSON on stdout, but cargo's own failures
/// never reach it: a build script that exits non-zero, an unresolvable
/// dependency, a malformed manifest, or a `--bin` that names no target are
/// only ever written to stderr as text, leaving stdout without a single
/// error-level diagnostic (and often empty altogether). Which stream holds
/// the failure is therefore decided by whether rustc reported an error at
/// all, rather than by how the build was invoked.
fn failure_diagnostics(messages: &[serde_json::Value], stderr: &str) -> String {
    let diagnostics = if has_compiler_error(messages) {
        rendered_diagnostics(messages)
    } else {
        cargo_error_output(stderr)
    };
    diagnostics.trim_end().to_string()
}

/// Whether rustc reported an error, as opposed to only warnings or nothing at
/// all. Every level it fails a build with starts with `error`: plain `error`,
/// and `error: internal compiler error` for an ICE.
fn has_compiler_error(messages: &[serde_json::Value]) -> bool {
    messages.iter().any(|msg| {
        is_compiler_message(msg)
            && msg
                .get("message")
                .and_then(|m| m.get("level"))
                .and_then(|l| l.as_str())
                .is_some_and(|level| level.starts_with("error"))
    })
}

/// The rendered text of every diagnostic rustc reported, in order. Warnings
/// are kept alongside the errors, matching what a plain `cargo build` prints.
fn rendered_diagnostics(messages: &[serde_json::Value]) -> String {
    messages
        .iter()
        .filter(|msg| is_compiler_message(msg))
        .filter_map(|msg| {
            msg.get("message")
                .and_then(|m| m.get("rendered"))
                .and_then(|r| r.as_str())
        })
        .collect()
}

fn is_compiler_message(msg: &serde_json::Value) -> bool {
    msg.get("reason").and_then(|r| r.as_str()) == Some("compiler-message")
}

/// Cargo's own error report, extracted from the stderr it interleaves with
/// status lines and progress bar renders.
///
/// Cargo redraws the progress bar in place with carriage returns rather than
/// newlines, so only the text after the last `\r` of a line was ever visible;
/// the report itself runs from the first `error` line to the end of the
/// stream. When there is no such line the build died without reporting
/// anything (killed by a signal, say), and the status lines are all there is
/// to go on.
fn cargo_error_output(stderr: &str) -> String {
    let lines: Vec<String> = stderr
        .split('\n')
        .map(|line| {
            let visible = line.rsplit('\r').next().unwrap_or_default();
            strip_ansi_codes(visible).trim_end().to_string()
        })
        .collect();

    let report = match lines.iter().position(|line| line.starts_with("error")) {
        Some(start) => lines[start..].join("\n"),
        None => lines
            .iter()
            .filter(|line| !line.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    };
    report.trim_end().to_string()
}

/// Pick the final linked outputs out of cargo's compiler-artifact messages:
/// bin executables plus `cdylib`/`dylib` library outputs.
///
/// Cargo emits an artifact message for every crate it compiles, but marks the
/// final outputs itself: `executable` is only set for the requested bin
/// targets, and only the requested packages' library outputs are uplifted out
/// of `deps/` into the profile directory. Everything still in `deps/` is an
/// intermediate dependency and is skipped.
fn select_artifacts(messages: &[serde_json::Value]) -> Vec<PathBuf> {
    let mut artifacts = Vec::new();
    for msg in messages {
        if msg.get("reason").and_then(|r| r.as_str()) != Some("compiler-artifact") {
            continue;
        }
        if let Some(executable) = msg.get("executable").and_then(|e| e.as_str()) {
            artifacts.push(PathBuf::from(executable));
        } else if is_dynamic_library_target(msg) {
            artifacts.extend(final_library_filenames(msg));
        }
    }
    artifacts
}

/// Whether `msg` is a compiler-artifact for a `cdylib` or `dylib` target, as
/// opposed to a plain `lib` or a proc-macro (whose shared object is uplifted
/// too when it is a requested target, but is not a scannable application).
fn is_dynamic_library_target(msg: &serde_json::Value) -> bool {
    msg.get("target")
        .and_then(|target| target.get("crate_types"))
        .and_then(|types| types.as_array())
        .is_some_and(|types| {
            types
                .iter()
                .any(|ty| matches!(ty.as_str(), Some("cdylib" | "dylib")))
        })
}

/// The final linked library outputs listed in a compiler-artifact's
/// `filenames`.
fn final_library_filenames(msg: &serde_json::Value) -> impl Iterator<Item = PathBuf> + '_ {
    msg.get("filenames")
        .and_then(|filenames| filenames.as_array())
        .into_iter()
        .flatten()
        .filter_map(|filename| filename.as_str())
        .filter(|filename| is_final_library(filename))
        .map(PathBuf::from)
}

/// Whether `path` names a final dynamic-library output: a linked library by
/// extension (as opposed to an rlib, import library, or dep-info companion in
/// the same `filenames` list) that cargo uplifted out of the `deps/`
/// directory, which it only does for the build's requested targets.
fn is_final_library(path: &str) -> bool {
    let path = Path::new(path);
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("wasm" | "so" | "dylib" | "dll")
    ) && path
        .parent()
        .and_then(Path::file_name)
        .is_none_or(|dir| dir != "deps")
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
    use super::*;
    use serde_json::json;

    fn diagnostic(level: &str, rendered: &str) -> serde_json::Value {
        json!({
            "reason": "compiler-message",
            "message": { "level": level, "rendered": rendered },
        })
    }

    #[test]
    fn rustc_errors_are_reported_from_the_json_stream() {
        let messages = vec![
            diagnostic("warning", "warning: unused variable `x`\n"),
            diagnostic("error", "error[E0308]: mismatched types\n"),
        ];
        assert_eq!(
            failure_diagnostics(
                &messages,
                "   Compiling app v0.1.0\nerror: could not compile `app` (bin \"app\") due to 1 previous error\n"
            ),
            "warning: unused variable `x`\nerror[E0308]: mismatched types"
        );
    }

    #[test]
    fn build_script_failures_are_reported_from_cargo_stderr() {
        // A build script that exits non-zero is cargo's own failure, not
        // rustc's: stdout carries the build script's artifact and nothing
        // else, so there is no diagnostic to render.
        let messages = vec![
            json!({
                "reason": "compiler-artifact",
                "target": { "crate_types": ["bin"] },
                "executable": null,
                "filenames": ["/target/debug/build/app-abc/build-script-build"],
            }),
            json!({ "reason": "build-finished", "success": false }),
        ];
        let stderr = concat!(
            "   Compiling app v0.1.0 (/app)\n",
            "    Building [=>     ] 0/3: app(build.rs)  \r",
            "    Building [====>  ] 1/3: app(build)     \r",
            "error: failed to run custom build command for `app v0.1.0 (/app)`\n",
            "\n",
            "Caused by:\n",
            "  process didn't exit successfully: `build-script-build` (exit status: 1)\n",
            "  --- stderr\n",
            "  something went wrong in the build script\n",
        );
        assert_eq!(
            failure_diagnostics(&messages, stderr),
            concat!(
                "error: failed to run custom build command for `app v0.1.0 (/app)`\n",
                "\n",
                "Caused by:\n",
                "  process didn't exit successfully: `build-script-build` (exit status: 1)\n",
                "  --- stderr\n",
                "  something went wrong in the build script",
            )
        );
    }

    #[test]
    fn manifest_errors_are_reported_with_an_empty_json_stream() {
        // Cargo fails before compiling anything, so stdout stays empty.
        let stderr = "error: unclosed table, expected `]`\n --> Cargo.toml:8:14\n";
        assert_eq!(
            failure_diagnostics(&[], stderr),
            "error: unclosed table, expected `]`\n --> Cargo.toml:8:14"
        );
    }

    #[test]
    fn warnings_alone_do_not_stand_in_for_a_cargo_failure() {
        // rustc compiled a dependency with warnings before cargo failed on
        // its own; reporting only the warnings would bury the error.
        let messages = vec![diagnostic("warning", "warning: unused import\n")];
        let stderr = "   Compiling dep v0.1.0\nerror: no bin target named `nope`\n";
        assert_eq!(
            failure_diagnostics(&messages, stderr),
            "error: no bin target named `nope`"
        );
    }

    #[test]
    fn progress_renders_and_color_are_stripped_from_cargo_output() {
        let stderr = "    Building [==>  ] 1/2: app  \r\u{1b}[1m\u{1b}[31merror\u{1b}[0m: boom\n";
        assert_eq!(cargo_error_output(stderr), "error: boom");
    }

    #[test]
    fn a_build_that_reports_no_error_falls_back_to_status_lines() {
        // Nothing to key off when cargo is killed outright, so report what it
        // managed to say rather than nothing at all.
        let stderr = "   Compiling app v0.1.0 (/app)\n";
        assert_eq!(cargo_error_output(stderr), "   Compiling app v0.1.0 (/app)");
    }

    fn artifact(crate_types: &[&str], filenames: &[&str]) -> serde_json::Value {
        json!({
            "reason": "compiler-artifact",
            "target": { "crate_types": crate_types },
            "executable": null,
            "filenames": filenames,
        })
    }

    #[test]
    fn executables_are_kept_and_noise_ignored() {
        let messages = vec![
            json!({ "reason": "build-script-executed" }),
            json!({
                "reason": "compiler-artifact",
                "target": { "crate_types": ["bin"] },
                "executable": "/target/debug/app",
                "filenames": ["/target/debug/app"],
            }),
        ];
        assert_eq!(
            select_artifacts(&messages),
            vec![PathBuf::from("/target/debug/app")]
        );
    }

    #[test]
    fn cdylib_output_is_picked_over_its_rlib_companion() {
        let messages = vec![artifact(
            &["cdylib", "rlib"],
            &["/target/wasm/app.wasm", "/target/wasm/libapp.rlib"],
        )];
        assert_eq!(
            select_artifacts(&messages),
            vec![PathBuf::from("/target/wasm/app.wasm")]
        );
    }

    #[test]
    fn dependency_cdylib_outputs_are_excluded() {
        // A dependency that declares `cdylib` produces a scannable output
        // too, but cargo leaves it in `deps/`; only the uplifted output of
        // the crate being built belongs in the bundle.
        let messages = vec![
            artifact(&["cdylib", "rlib"], &["/target/deps/libdep.dylib"]),
            artifact(&["cdylib"], &["/target/libapp.dylib"]),
        ];
        assert_eq!(
            select_artifacts(&messages),
            vec![PathBuf::from("/target/libapp.dylib")]
        );
    }

    #[test]
    fn multiple_final_library_outputs_are_all_kept() {
        let messages = vec![
            artifact(&["cdylib"], &["/target/liba.dylib"]),
            artifact(&["cdylib"], &["/target/libb.dylib"]),
        ];
        // Building several packages with library outputs at once uplifts each
        // of them, so `build` reports the ambiguity and asks the user to pass
        // `--package`.
        assert_eq!(select_artifacts(&messages).len(), 2);
    }

    #[test]
    fn proc_macro_libraries_are_not_scanned() {
        // A workspace build uplifts the proc-macro members' shared objects
        // right next to the application's output.
        let messages = vec![artifact(&["proc-macro"], &["/target/libmacros.dylib"])];
        assert!(select_artifacts(&messages).is_empty());
    }

    #[test]
    fn plain_library_dependencies_are_not_scanned() {
        let messages = vec![artifact(&["lib"], &["/target/deps/libdep.rlib"])];
        assert!(select_artifacts(&messages).is_empty());
    }

    #[test]
    fn recognizes_final_libraries() {
        assert!(is_final_library("/t/wasm32-unknown-unknown/debug/app.wasm"));
        assert!(is_final_library("/t/debug/libapp.so"));
        assert!(is_final_library("/t/debug/libapp.dylib"));
        assert!(is_final_library("/t/debug/app.dll"));
        assert!(!is_final_library("/t/debug/deps/app.wasm"));
        assert!(!is_final_library("/t/debug/deps/libapp.dylib"));
        assert!(!is_final_library("/t/debug/libapp.rlib"));
        assert!(!is_final_library("/t/debug/app.dll.lib"));
        assert!(!is_final_library("/t/debug/app.d"));
    }
}
