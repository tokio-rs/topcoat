//! The `topcoat dev` command: an auto-rebuilding development server.
//!
//! Four pieces cooperate, tied together by the event loop in
//! [`DevCommand::run`]:
//!
//! - [`broadcast_server`]: a long-lived local WebSocket server that browsers connect to; it
//!   broadcasts a reload message whenever a freshly started application reports ready.
//! - [`watch`]: watches the workspace's source directories and coalesces bursts of filesystem
//!   events into single change notifications.
//! - [`build`]: compiles the application and bundles its assets in a cancellable background task.
//! - [`app_server`]: the application process itself.
//!
//! The loop's core policy is that the running application is only ever
//! replaced by a *successful* build: while a rebuild is in flight, and after
//! a failed one, the previous process keeps serving.

mod app_server;
mod broadcast_server;
mod build;
mod spinner;
mod watch;

use std::path::Path;

use clap::Args;
use console::style;

use app_server::AppServer;
use broadcast_server::{Event, EventBus};
use build::{BuildKind, BuildTask};
use spinner::Spinner;
use watch::SourceWatcher;

#[derive(Args)]
pub struct DevCommand {}

impl DevCommand {
    pub async fn run(self) {
        // The broadcast server outlives the individual application
        // processes: browsers stay connected to it across rebuilds.
        let listener = broadcast_server::bind().await;
        let dev_url = format!("http://{}", listener.local_addr().unwrap());
        let events = EventBus::new();
        tokio::spawn(broadcast_server::run(listener, events.clone()));

        eprintln!();
        eprintln!(
            "  {} {}",
            style("topcoat").cyan().bold(),
            style("dev server").dim()
        );
        let mut watcher = SourceWatcher::start().await;
        eprintln!("  {}", style("watching for file changes...").dim());
        eprintln!();

        let mut build: Option<BuildTask> = Some(BuildTask::spawn(BuildKind::Initial));
        let mut server: Option<AppServer> = None;
        events.publish(Event::Rebuilding);

        loop {
            // `select!` evaluates every branch expression even when its `if`
            // guard is false (it just never polls the future), so the
            // `unwrap`s must sit inside lazy `async` blocks.
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    eprintln!();
                    let spinner = Spinner::new("shutting down...");
                    if let Some(build) = build.take() {
                        build.cancel().await;
                    }
                    if let Some(server) = server.take() {
                        server.shutdown().await;
                    }
                    drop(spinner);
                    eprintln!();
                    break;
                }

                exe = async { build.as_mut().unwrap().finished().await }, if build.is_some() => {
                    build = None;
                    if let Some(exe) = exe {
                        // The new process binds the same address as the old
                        // one, so the old one must be stopped first.
                        if let Some(old) = server.take() {
                            old.shutdown().await;
                        }
                        server = start_app(&exe, &dev_url, &events);
                    } else {
                        // The failure is already on the terminal; keep the
                        // previous process serving while the user fixes it.
                        events.publish(Event::BuildFailed);
                        report_waiting(server.is_some());
                    }
                }

                status = async { server.as_mut().unwrap().exited().await }, if server.is_some() => {
                    server = None;
                    events.publish(Event::AppExited);
                    let status = status.map_or_else(|error| error.to_string(), |status| status.to_string());
                    eprintln!(
                        "  {}",
                        style(format!("application exited ({status})")).red().bold()
                    );
                    eprintln!();
                    report_waiting(false);
                }

                () = watcher.changed() => {
                    // Rebuild, but leave the running application untouched:
                    // it keeps serving until the new build is ready. A stale
                    // in-flight build compiles sources that just changed
                    // again, so cancel it rather than wait for it.
                    if let Some(stale) = build.take() {
                        stale.cancel().await;
                    }
                    build = Some(BuildTask::spawn(BuildKind::Rebuild));
                    events.publish(Event::Rebuilding);
                }
            }
        }
    }
}

/// Start the built executable, reporting a failure to the terminal and to
/// connected browsers.
fn start_app(exe: &Path, dev_url: &str, events: &EventBus) -> Option<AppServer> {
    match AppServer::spawn(exe, dev_url) {
        Ok(server) => Some(server),
        Err(error) => {
            events.publish(Event::AppExited);
            eprintln!(
                "  {}",
                style(format!("failed to start application: {error}"))
                    .red()
                    .bold()
            );
            eprintln!();
            report_waiting(false);
            None
        }
    }
}

/// Print the idle status line shown after a failure.
fn report_waiting(server_running: bool) {
    let message = if server_running {
        "previous build still running; waiting for changes..."
    } else {
        "waiting for changes..."
    };
    eprintln!("  {}", style(message).dim());
    eprintln!();
}
