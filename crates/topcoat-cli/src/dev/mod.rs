mod broadcast_server;

use clap::Args;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use notify::{RecursiveMode, Watcher, recommended_watcher};
use std::error::Error;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::Duration;

#[derive(Args)]
pub struct DevCommand {}

type BuildFut = Pin<Box<dyn Future<Output = Option<Child>> + Send>>;

impl DevCommand {
    pub async fn run(self) {
        let listener = broadcast_server::bind().await;
        let dev_url = format!("ws://{}", listener.local_addr().unwrap());
        tokio::spawn(broadcast_server::run(listener));

        eprintln!();
        eprintln!(
            "  {} {}",
            style("topcoat").cyan().bold(),
            style("dev server").dim()
        );
        let watch_dirs = discover_watch_dirs().await;
        eprintln!("  {}", style("watching for file changes...").dim());
        eprintln!();

        let mut current_build: Option<BuildFut> = Some(spawn_build(true, &dev_url));
        let mut child: Option<Child> = None;

        let (tx, mut rx) = mpsc::channel::<notify::Result<notify::Event>>(16);
        let mut watcher = recommended_watcher(move |event| {
            let _ = tx.blocking_send(event);
        })
        .expect("failed to create file watcher");
        for dir in &watch_dirs {
            watcher
                .watch(dir, RecursiveMode::Recursive)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "  {}",
                        style(format!("failed to watch {}: {e}", dir.display())).yellow()
                    )
                });
        }

        let debounce = Duration::from_millis(200);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    eprintln!();
                    let spinner = make_spinner("shutting down...");
                    eprintln!();
                    drop(current_build.take());
                    if let Some(c) = &mut child {
                        kill_child(c).await;
                    }
                    spinner.finish_and_clear();
                    eprintln!();
                    break;
                }
                result = wait_for_build(&mut current_build) => {
                    current_build = None;
                    child = result;
                }
                Some(_event) = rx.recv() => {
                    while rx.try_recv().is_ok() {}
                    tokio::time::sleep(debounce).await;
                    while rx.try_recv().is_ok() {}

                    drop(current_build.take());
                    if let Some(c) = &mut child {
                        kill_child(c).await;
                    }
                    child = None;
                    current_build = Some(spawn_build(false, &dev_url));
                }
            }
        }
    }
}

fn spawn_build(initial: bool, dev_url: &str) -> BuildFut {
    let dev_url = dev_url.to_string();
    Box::pin(async move { build_and_run(initial, &dev_url).await })
}

async fn wait_for_build(slot: &mut Option<BuildFut>) -> Option<Child> {
    match slot {
        Some(fut) => fut.as_mut().await,
        None => std::future::pending().await,
    }
}

async fn discover_watch_dirs() -> Vec<PathBuf> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .expect("failed to run cargo metadata");

    if !output.status.success() {
        eprintln!("  cargo metadata failed, falling back to ./src");
        return vec![PathBuf::from("./src")];
    }

    let meta: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("failed to parse cargo metadata");

    let dirs: Vec<PathBuf> = meta["packages"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|pkg| {
            let manifest = PathBuf::from(pkg["manifest_path"].as_str()?);
            let src = manifest.parent()?.join("src");
            src.is_dir().then_some(src)
        })
        .collect();

    if dirs.is_empty() {
        vec![PathBuf::from("./src")]
    } else {
        dirs
    }
}

fn make_spinner(message: &str) -> ProgressBar {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("  {spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.set_message(message.to_string());
    spinner.enable_steady_tick(Duration::from_millis(80));
    spinner
}

async fn build_and_run(initial: bool, dev_url: &str) -> Option<Child> {
    let label = if initial { "building" } else { "rebuilding" };
    let spinner = make_spinner(label);

    let output = Command::new("cargo")
        .args(["build", "--message-format=json-diagnostic-rendered-ansi"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .expect("failed to spawn cargo build")
        .wait_with_output()
        .await
        .expect("failed to wait for cargo build");

    spinner.finish_and_clear();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let messages: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if !output.status.success() {
        eprintln!("  {}", style("build failed").red().bold());
        eprintln!();

        for msg in &messages {
            if let Some(rendered) = msg
                .get("message")
                .and_then(|m| m.get("rendered"))
                .and_then(|r| r.as_str())
            {
                eprint!("{rendered}");
            }
        }

        eprintln!();
        eprintln!("  {}", style("waiting for changes...").dim());
        eprintln!();
        return None;
    }

    let executable = messages.iter().find_map(|msg| {
        if msg.get("reason")?.as_str()? == "compiler-artifact" && msg.get("executable").is_some() {
            msg["executable"].as_str().map(String::from)
        } else {
            None
        }
    });

    let spinner = make_spinner("bundling assets");

    let Some(exe) = executable else {
        eprintln!("  {}", style("could not determine executable path").red());
        return None;
    };

    if let Err(err) = bundle_assets(&exe) {
        spinner.finish_and_clear();
        eprintln!(
            "  {}",
            style(format!("failed to bundle assets: {err}"))
                .red()
                .bold()
        );
        eprintln!();
        return None;
    }

    spinner.finish_and_clear();

    eprintln!("  {}", style("ready").green().bold());
    eprintln!();

    Some(
        Command::new(&exe)
            .env("TOPCOAT_DEV_URL", dev_url)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()
            .expect("failed to spawn application"),
    )
}

async fn kill_child(child: &mut Child) {
    let _ = child.kill().await;
    let _ = child.wait().await;
}

fn bundle_assets(executable: &str) -> Result<(), Box<dyn Error>> {
    let exe = PathBuf::from(executable);
    let out_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not derive cargo target directory",
            )
        })?
        .join("assets");
    let bytes = std::fs::read(&exe)?;
    Ok(topcoat_asset::Bundler::bundle(&bytes, &out_dir)?)
}
