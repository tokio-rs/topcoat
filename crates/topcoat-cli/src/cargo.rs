use std::fmt;
use std::io;
use std::path::PathBuf;
use std::process::Stdio;

use console::style;
use tokio::process::Command;

pub async fn target_dir() -> Option<PathBuf> {
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

    let msg: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    msg.get("target_directory")?.as_str().map(PathBuf::from)
}

#[derive(Default)]
pub struct BuildOpts {
    pub bin: Option<String>,
    pub package: Option<String>,
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

pub async fn build(opts: &BuildOpts) -> Result<PathBuf, BuildError> {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--message-format=json-diagnostic-rendered-ansi"]);
    if let Some(bin) = &opts.bin {
        cmd.args(["--bin", bin]);
    }
    if let Some(package) = &opts.package {
        cmd.args(["--package", package]);
    }

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(BuildError::Spawn)?
        .wait_with_output()
        .await
        .map_err(BuildError::Spawn)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let messages: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if !output.status.success() {
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

pub async fn build_and_read(opts: &BuildOpts) -> Result<(PathBuf, Vec<u8>), BuildError> {
    let path = build(opts).await?;
    let bytes = std::fs::read(&path).map_err(BuildError::Read)?;
    Ok((path, bytes))
}
