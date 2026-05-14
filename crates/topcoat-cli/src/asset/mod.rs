use std::path::PathBuf;
use std::process::Stdio;

use clap::{Args, Subcommand};
use console::style;
use tokio::process::Command;

#[derive(Args)]
pub struct AssetCommand {
    #[command(subcommand)]
    command: AssetSubcommand,
}

#[derive(Subcommand)]
enum AssetSubcommand {
    /// List all asset paths embedded in the binary produced by cargo
    List(ListArgs),
    /// Bundle all assets embedded in the binary into a directory
    Bundle(BundleArgs),
    /// Delete the asset bundle directory and the asset build cache
    Clean(CleanArgs),
}

#[derive(Args)]
struct ListArgs {
    /// Build and inspect the named binary target
    #[arg(long)]
    bin: Option<String>,
    /// Build and inspect the named package
    #[arg(short, long)]
    package: Option<String>,
}

#[derive(Args)]
struct BundleArgs {
    /// Build and inspect the named binary target
    #[arg(long)]
    bin: Option<String>,
    /// Build and inspect the named package
    #[arg(short, long)]
    package: Option<String>,
    /// Output directory for the bundle (defaults to <cargo-target>/assets)
    #[arg(short, long)]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct CleanArgs {
    /// Asset bundle directory to remove (defaults to <cargo-target>/assets)
    #[arg(short, long)]
    out: Option<PathBuf>,
}

impl AssetCommand {
    pub async fn run(self) {
        match self.command {
            AssetSubcommand::List(args) => list(args).await,
            AssetSubcommand::Bundle(args) => bundle(args).await,
            AssetSubcommand::Clean(args) => clean(args).await,
        }
    }
}

async fn list(args: ListArgs) {
    let executable = match build_executable(args.bin.as_deref(), args.package.as_deref()).await {
        Some(path) => path,
        None => std::process::exit(1),
    };

    let bytes = match std::fs::read(&executable) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!(
                "{}",
                style(format!("failed to read {executable}: {error}")).red()
            );
            std::process::exit(1);
        }
    };

    for asset in topcoat_asset::RawAsset::find_in_binary(&bytes) {
        match asset.source() {
            topcoat_asset::Source::Path(p) => {
                println!("{}", p.to_str().unwrap_or("<non-utf8 file path>"))
            }
            topcoat_asset::Source::Url(uri) => println!("{uri}"),
        }
    }
}

async fn bundle(args: BundleArgs) {
    let executable = match build_executable(args.bin.as_deref(), args.package.as_deref()).await {
        Some(path) => path,
        None => std::process::exit(1),
    };

    let bytes = match std::fs::read(&executable) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!(
                "{}",
                style(format!("failed to read {executable}: {error}")).red()
            );
            std::process::exit(1);
        }
    };

    let target_dir = match cargo_target_dir().await {
        Some(path) => path,
        None => {
            eprintln!(
                "{}",
                style("could not derive cargo target directory; pass --out").red()
            );
            std::process::exit(1);
        }
    };

    let out_dir = args.out.unwrap_or_else(|| target_dir.join("assets"));
    let cache_dir = target_dir.join("topcoat/cache/assets");

    let bundler = topcoat_asset::Bundler::new(cache_dir);
    if let Err(error) = bundler.bundle(&bytes, &out_dir).await {
        eprintln!(
            "{}",
            style(format!("failed to bundle assets: {error}")).red()
        );
        std::process::exit(1);
    }

    println!("bundled assets into {}", out_dir.display());
}

async fn clean(args: CleanArgs) {
    let target_dir = match cargo_target_dir().await {
        Some(path) => path,
        None => {
            eprintln!(
                "{}",
                style("could not derive cargo target directory; pass --out").red()
            );
            std::process::exit(1);
        }
    };

    let out_dir = args.out.unwrap_or_else(|| target_dir.join("assets"));
    let cache_dir = target_dir.join("topcoat/cache/assets");

    for dir in [&out_dir, &cache_dir] {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => println!("removed {}", dir.display()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                eprintln!(
                    "{}",
                    style(format!("failed to remove {}: {error}", dir.display())).red()
                );
                std::process::exit(1);
            }
        }
    }
}

async fn cargo_target_dir() -> Option<PathBuf> {
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

async fn build_executable(bin: Option<&str>, package: Option<&str>) -> Option<String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "--message-format=json"]);
    if let Some(bin) = bin {
        cmd.args(["--bin", bin]);
    }
    if let Some(package) = package {
        cmd.args(["--package", package]);
    }

    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("failed to spawn cargo build")
        .wait_with_output()
        .await
        .expect("failed to wait for cargo build");

    if !output.status.success() {
        eprintln!("{}", style("build failed").red().bold());
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let executables: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let msg: serde_json::Value = serde_json::from_str(line).ok()?;
            if msg.get("reason")?.as_str()? == "compiler-artifact" {
                msg.get("executable")?.as_str().map(String::from)
            } else {
                None
            }
        })
        .collect();

    match executables.len() {
        0 => {
            eprintln!("{}", style("no executable produced by cargo build").red());
            None
        }
        1 => Some(executables.into_iter().next().unwrap()),
        _ => {
            eprintln!(
                "{}",
                style("cargo produced multiple binaries; pass --bin or --package to choose one:")
                    .red()
            );
            for exe in &executables {
                eprintln!("  {exe}");
            }
            None
        }
    }
}
