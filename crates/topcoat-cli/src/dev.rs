mod broadcast_server;

use clap::Args;
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::Duration;

use crate::cargo::{BuildError, BuildOpts};

#[derive(Args)]
pub struct DevCommand {}

type BuildFut = Pin<Box<dyn Future<Output = Option<Child>> + Send>>;

impl DevCommand {
    pub async fn run(self) {
        let listener = broadcast_server::bind().await;
        let dev_url = format!("http://{}", listener.local_addr().unwrap());
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

        let (tx, mut rx) = mpsc::unbounded_channel::<notify::Result<notify::Event>>();
        let mut watcher = recommended_watcher(move |event: notify::Result<notify::Event>| {
            if let Ok(ev) = &event
                && !matches!(ev.kind, EventKind::Access(_))
            {
                let _ = tx.send(event);
            }
        })
        .expect("failed to create file watcher");
        for dir in &watch_dirs {
            watcher
                .watch(dir, RecursiveMode::Recursive)
                .unwrap_or_else(|e| {
                    eprintln!(
                        "  {}",
                        style(format!("failed to watch {}: {e}", dir.display())).yellow()
                    );
                });
        }

        let debounce = Duration::from_millis(200);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    eprintln!();
                    let spinner = Spinner::new("shutting down...");
                    eprintln!();
                    drop(current_build.take());
                    if let Some(c) = &mut child {
                        kill_child(c).await;
                    }
                    drop(spinner);
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

struct Spinner(ProgressBar);

impl Spinner {
    fn new(message: &str) -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("  {spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(80));
        Self(spinner)
    }
}

impl std::ops::Deref for Spinner {
    type Target = ProgressBar;
    fn deref(&self) -> &ProgressBar {
        &self.0
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.0.finish_and_clear();
    }
}

async fn build_and_run(initial: bool, dev_url: &str) -> Option<Child> {
    let label = if initial { "building" } else { "rebuilding" };
    let spinner = Spinner::new(label);
    let progress_spinner = spinner.clone();
    let result = crate::cargo::build_and_read(&BuildOpts::default(), move |cur, total| {
        progress_spinner.set_message(format!("{label} ({cur}/{total})"));
    })
    .await;
    drop(spinner);

    let (exe, bytes) = match result {
        Ok(pair) => pair,
        Err(error) => {
            eprintln!("  {}", style("build failed").red().bold());
            eprintln!();
            if let BuildError::Failed { rendered } = &error {
                eprint!("{rendered}");
            } else {
                eprintln!("  {}", style(error.to_string()).red());
            }
            eprintln!();
            eprintln!("  {}", style("waiting for changes...").dim());
            eprintln!();
            return None;
        }
    };

    let spinner = Spinner::new("bundling assets");
    let bundle_result = crate::asset::run_bundle(&bytes, None).await;
    drop(spinner);

    if let Err(err) = bundle_result {
        eprintln!(
            "  {}",
            style(format!("failed to bundle assets: {err}"))
                .red()
                .bold()
        );
        eprintln!();
        return None;
    }

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
