use std::io;
use std::path::Path;
use std::process::{ExitStatus, Stdio};

use tokio::process::{Child, Command};

/// A running instance of the application under development.
///
/// The process inherits the terminal's stdio and receives the broadcast
/// server's URL through the `TOPCOAT_DEV_URL` environment variable, which
/// the framework uses to report readiness and to inject the reload script.
pub struct AppServer {
    child: Child,
}

impl AppServer {
    /// Run the built executable.
    pub fn spawn(exe: &Path, dev_url: &str) -> io::Result<Self> {
        let child = Command::new(exe)
            .env("TOPCOAT_DEV_URL", dev_url)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::inherit())
            // A safety net for abnormal exits; the regular paths stop the
            // process explicitly via `shutdown`.
            .kill_on_drop(true)
            .spawn()?;
        Ok(Self { child })
    }

    /// Wait for the application to exit on its own.
    ///
    /// Cancel-safe: cancelling loses no state, and a later call resolves
    /// with the same exit status.
    pub async fn exited(&mut self) -> io::Result<ExitStatus> {
        self.child.wait().await
    }

    /// Kill the application and wait for the process to be reaped.
    pub async fn shutdown(mut self) {
        // `kill` also reaps the process on success, but when the process has
        // already exited it can return an error without reaping, so follow
        // up with an explicit `wait`.
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}
