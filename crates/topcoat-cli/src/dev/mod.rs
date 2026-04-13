use clap::Args;
use indicatif::ProgressBar;
use notify::{RecursiveMode, Watcher, recommended_watcher};
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::time::{Duration, Instant};

#[derive(Args)]
pub struct DevCommand {}

impl DevCommand {
    pub async fn run(self) {
        let mut child = build_and_run().await;

        let (tx, mut rx) = mpsc::channel::<notify::Result<notify::Event>>(16);
        let mut watcher = recommended_watcher(move |event| {
            let _ = tx.blocking_send(event);
        })
        .expect("failed to create file watcher");
        watcher
            .watch(std::path::Path::new("./src"), RecursiveMode::Recursive)
            .expect("failed to watch ./src directory");

        let debounce = Duration::from_millis(200);
        let mut last_rebuild = Instant::now();

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    eprintln!("\nShutting down...");
                    if let Some(c) = &mut child {
                        kill_child(c).await;
                    }
                    break;
                }
                Some(_event) = rx.recv() => {
                    // Drain any extra queued events
                    while rx.try_recv().is_ok() {}

                    if last_rebuild.elapsed() < debounce {
                        continue;
                    }

                    // Debounce: wait a short period to batch rapid saves
                    tokio::time::sleep(debounce).await;
                    // Drain events accumulated during the debounce sleep
                    while rx.try_recv().is_ok() {}

                    last_rebuild = Instant::now();
                    if let Some(c) = &mut child {
                        kill_child(c).await;
                    }
                    child = build_and_run().await;
                }
            }
        }
    }
}

/// Builds the project silently with a spinner, then runs the binary directly.
/// Returns `None` if the build failed (errors are printed to stderr).
async fn build_and_run() -> Option<Child> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_message("Building...");
    spinner.enable_steady_tick(Duration::from_millis(80));

    let output = Command::new("cargo")
        .args(["build", "--message-format=json-diagnostic-rendered-ansi"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn cargo build")
        .wait_with_output()
        .await
        .expect("failed to wait for cargo build");

    spinner.finish_and_clear();

    // Parse cargo's JSON messages
    let stdout = String::from_utf8_lossy(&output.stdout);
    let messages: Vec<serde_json::Value> = stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    if !output.status.success() {
        // Print rendered compiler diagnostics
        for msg in &messages {
            if let Some(rendered) = msg
                .get("message")
                .and_then(|m| m.get("rendered"))
                .and_then(|r| r.as_str())
            {
                eprint!("{rendered}");
            }
        }
        return None;
    }

    let executable = messages.iter().find_map(|msg| {
        if msg.get("reason")?.as_str()? == "compiler-artifact"
            && msg.get("executable").is_some()
        {
            msg["executable"].as_str().map(String::from)
        } else {
            None
        }
    });

    let Some(exe) = executable else {
        eprintln!("Could not determine executable path from cargo build output");
        return None;
    };

    Some(
        Command::new(exe)
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
