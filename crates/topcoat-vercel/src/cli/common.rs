use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
pub(super) struct Metadata {
    pub(super) packages: Vec<Package>,
    pub(super) target_directory: PathBuf,
}

impl Metadata {
    pub(super) fn load() -> Result<Self> {
        let output = cargo(["metadata", "--format-version", "1", "--no-deps"])?;
        serde_json::from_slice(&output.stdout).context("failed to read cargo metadata")
    }

    pub(super) fn current_package(&self) -> Result<&Package> {
        let output = cargo(["locate-project", "--message-format", "plain"])?;
        let manifest = Path::new(
            std::str::from_utf8(&output.stdout)
                .context("cargo returned a non-UTF-8 manifest path")?
                .trim(),
        )
        .canonicalize()
        .context("failed to resolve the current Cargo manifest")?;

        self.packages
            .iter()
            .find(|package| {
                package
                    .manifest_path
                    .canonicalize()
                    .is_ok_and(|path| path == manifest)
            })
            .context("the current Cargo manifest is not a package")
    }
}

#[derive(Deserialize)]
pub(super) struct Package {
    pub(super) default_run: Option<String>,
    pub(super) manifest_path: PathBuf,
    pub(super) targets: Vec<Target>,
}

impl Package {
    pub(super) fn binary(&self, requested: Option<&str>) -> Result<&str> {
        let binaries = self
            .targets
            .iter()
            .filter(|target| target.kind.iter().any(|kind| kind == "bin"))
            .collect::<Vec<_>>();

        if let Some(requested) = requested {
            return binaries
                .into_iter()
                .find(|target| target.name == requested)
                .map(|target| target.name.as_str())
                .with_context(|| format!("package has no `{requested}` binary"));
        }

        if let Some(default) = self.default_run.as_deref() {
            return Ok(default);
        }

        match binaries.as_slice() {
            [binary] => Ok(&binary.name),
            [] => bail!("package has no binary target"),
            _ => bail!("package has several binaries; pass `--bin <name>`"),
        }
    }
}

#[derive(Deserialize)]
pub(super) struct Target {
    pub(super) kind: Vec<String>,
    pub(super) name: String,
}

pub(super) fn cargo<const N: usize>(args: [&str; N]) -> Result<Output> {
    let mut command = Command::new(env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    if env::var_os("VERCEL").is_some() {
        command.arg(concat!("+", env!("CARGO_PKG_RUST_VERSION")));
    }
    let output = command.args(args).output().context("failed to run cargo")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("cargo failed: {}", stderr.trim());
    }
    Ok(output)
}

pub(super) fn copy_dir(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {}", destination.display()))?;

    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let source = entry.path();
        let destination = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&source, &destination)?;
        } else {
            fs::copy(&source, &destination).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
    }
    Ok(())
}
