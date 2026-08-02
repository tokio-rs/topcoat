use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use clap::Args;
use serde_json::json;
use topcoat_asset::{Bundler, BundlerConfig, MANIFEST_NAME};

use super::common::{Metadata, cargo, copy_dir};

const ASSET_CACHE_SCOPE: &str = "assets";

#[derive(Args)]
pub(super) struct BuildCommand {
    /// Binary containing the Topcoat application
    #[arg(long)]
    bin: Option<String>,
}

impl BuildCommand {
    pub(super) fn run(self) -> Result<()> {
        ensure_build_platform()?;

        let metadata = Metadata::load()?;
        let binary = metadata.current_package()?.binary(self.bin.as_deref())?;
        cargo(["build", "--release", "--bin", binary])?;

        let executable = metadata.target_directory.join("release").join(binary);
        let assets = metadata.target_directory.join("release/assets");
        bundle_assets(&metadata.target_directory, &executable, &assets)?;
        write_output(Path::new(".vercel/output"), &executable, &assets)?;

        println!("built Vercel output in .vercel/output");
        Ok(())
    }
}

fn ensure_build_platform() -> Result<()> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Ok(())
    } else {
        bail!(
            "native Vercel builds require Linux x86_64; run this command in Vercel's build environment"
        )
    }
}

fn bundle_assets(target: &Path, executable: &Path, assets: &Path) -> Result<()> {
    let bytes =
        fs::read(executable).with_context(|| format!("failed to read {}", executable.display()))?;
    let cache = topcoat_core::cache::cache_dir_in(target, ASSET_CACHE_SCOPE);
    let config = BundlerConfig::new().cache_dir(cache);
    Bundler::new(&config)
        .bundle(&bytes, assets)
        .context("failed to bundle Topcoat assets")?;
    Ok(())
}

fn write_output(output: &Path, executable: &Path, assets: &Path) -> Result<()> {
    if output.exists() {
        fs::remove_dir_all(output)
            .with_context(|| format!("failed to clear {}", output.display()))?;
    }

    let function = output.join("functions/index.func");
    fs::create_dir_all(&function)
        .with_context(|| format!("failed to create {}", function.display()))?;
    let deployed_executable = function.join("executable");
    fs::copy(executable, &deployed_executable).with_context(|| {
        format!(
            "failed to copy {} to {}",
            executable.display(),
            deployed_executable.display()
        )
    })?;
    make_executable(&deployed_executable)?;
    copy_dir(assets, &function.join("assets"))?;

    let static_assets = output.join("static/_topcoat/assets");
    fs::create_dir_all(&static_assets)
        .with_context(|| format!("failed to create {}", static_assets.display()))?;
    for entry in
        fs::read_dir(assets).with_context(|| format!("failed to read {}", assets.display()))?
    {
        let entry = entry?;
        if entry.file_type()?.is_file() && entry.file_name() != MANIFEST_NAME {
            fs::copy(entry.path(), static_assets.join(entry.file_name()))?;
        }
    }

    write_json(
        &function.join(".vc-config.json"),
        &json!({
            "handler": "executable",
            "runtime": "executable",
            "runtimeLanguage": "rust",
            "architecture": "x86_64",
            "supportsResponseStreaming": true,
        }),
    )?;
    write_json(
        &output.join("config.json"),
        &json!({
            "version": 3,
            "routes": [
                { "handle": "filesystem" },
                { "src": "/(.*)", "dest": "/index" },
            ],
            "framework": {
                "version": env!("CARGO_PKG_VERSION"),
            },
        }),
    )
}

fn write_json(path: &Path, value: &serde_json::Value) -> Result<()> {
    let mut contents = serde_json::to_string_pretty(value)?;
    contents.push('\n');
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
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
                "topcoat-vercel-build-{}-{sequence}",
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
    fn writes_a_streaming_function_and_static_assets() {
        let root = TempDir::new();
        let executable = root.0.join("app");
        let assets = root.0.join("assets");
        let output = root.0.join("output");
        fs::write(&executable, "binary").unwrap();
        fs::create_dir(&assets).unwrap();
        fs::write(assets.join(MANIFEST_NAME), "version = 1\n").unwrap();
        fs::write(assets.join("app-123.css"), "body {}").unwrap();

        write_output(&output, &executable, &assets).unwrap();

        let function_config: serde_json::Value = serde_json::from_slice(
            &fs::read(output.join("functions/index.func/.vc-config.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(function_config["runtime"], "executable");
        assert_eq!(function_config["runtimeLanguage"], "rust");
        assert_eq!(function_config["supportsResponseStreaming"], true);
        assert!(
            output
                .join("functions/index.func/assets/manifest.toml")
                .is_file()
        );
        assert!(output.join("static/_topcoat/assets/app-123.css").is_file());
        assert!(!output.join("static/_topcoat/assets/manifest.toml").exists());

        let config: serde_json::Value =
            serde_json::from_slice(&fs::read(output.join("config.json")).unwrap()).unwrap();
        assert_eq!(config["version"], 3);
        assert_eq!(config["routes"][0]["handle"], "filesystem");
        assert_eq!(config["routes"][1]["dest"], "/index");
    }
}
