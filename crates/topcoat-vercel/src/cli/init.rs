use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde_json::json;

const RUST_VERSION: &str = "1.95.0";

#[derive(Args)]
pub(super) struct InitCommand {
    /// Binary to build when the package contains more than one
    #[arg(long)]
    bin: Option<String>,
    /// Replace an existing vercel.json
    #[arg(long)]
    force: bool,
}

impl InitCommand {
    pub(super) fn run(self) -> Result<()> {
        init(Path::new("."), self.bin.as_deref(), self.force)?;
        println!("created vercel.json");
        Ok(())
    }
}

fn init(root: &Path, binary: Option<&str>, force: bool) -> Result<()> {
    let config_path = root.join("vercel.json");
    if config_path.exists() && !force {
        bail!(
            "{} already exists; pass `--force` to replace it",
            config_path.display()
        );
    }

    let build_command = binary.map_or_else(
        || "topcoat-vercel build".to_owned(),
        |binary| format!("topcoat-vercel build --bin {binary}"),
    );
    let install_command = format!(
        "rustup toolchain install {RUST_VERSION} --profile minimal && \
         cargo +{RUST_VERSION} install topcoat-vercel --version {} --locked",
        env!("CARGO_PKG_VERSION")
    );
    let config = json!({
        "$schema": "https://openapi.vercel.sh/vercel.json",
        "framework": null,
        "installCommand": install_command,
        "buildCommand": build_command,
    });
    let mut contents = serde_json::to_string_pretty(&config)?;
    contents.push('\n');
    fs::write(&config_path, contents)
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    add_rust_toolchain(root)?;
    add_gitignore(root)
}

fn add_rust_toolchain(root: &Path) -> Result<()> {
    let path = root.join("rust-toolchain.toml");
    if path.exists() {
        return Ok(());
    }
    fs::write(
        path,
        format!("[toolchain]\nchannel = \"{RUST_VERSION}\"\nprofile = \"minimal\"\n"),
    )
    .context("failed to write rust-toolchain.toml")
}

fn add_gitignore(root: &Path) -> Result<()> {
    const ENTRY: &str = ".vercel/output/";
    let path = root.join(".gitignore");
    let mut contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error).context("failed to read .gitignore"),
    };
    if contents.lines().any(|line| line.trim() == ENTRY) {
        return Ok(());
    }
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(ENTRY);
    contents.push('\n');
    fs::write(path, contents).context("failed to update .gitignore")
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new() -> Self {
            let sequence = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "topcoat-vercel-init-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn writes_build_output_configuration() {
        let root = TempDir::new();
        init(&root.0, Some("store"), false).unwrap();

        let config: serde_json::Value =
            serde_json::from_slice(&fs::read(root.0.join("vercel.json")).unwrap()).unwrap();
        assert_eq!(config["buildCommand"], "topcoat-vercel build --bin store");
        assert!(config.get("outputDirectory").is_none());
        assert_eq!(
            fs::read_to_string(root.0.join(".gitignore")).unwrap(),
            ".vercel/output/\n"
        );
        assert_eq!(
            fs::read_to_string(root.0.join("rust-toolchain.toml")).unwrap(),
            "[toolchain]\nchannel = \"1.95.0\"\nprofile = \"minimal\"\n"
        );
    }

    #[test]
    fn preserves_an_existing_gitignore_entry() {
        let root = TempDir::new();
        fs::write(root.0.join(".gitignore"), "target/\n.vercel/output/\n").unwrap();

        init(&root.0, None, false).unwrap();

        assert_eq!(
            fs::read_to_string(root.0.join(".gitignore")).unwrap(),
            "target/\n.vercel/output/\n"
        );
    }
}
