use std::io;
use std::path::{Path, PathBuf};
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
    ///
    /// On Windows the executable is copied to a shadow path and the copy is
    /// run instead: a running process locks its image file, so launching the
    /// original would make every subsequent rebuild fail at the link step
    /// ("Access is denied") while the server keeps serving.
    pub fn spawn(exe: &Path, dev_url: &str) -> io::Result<Self> {
        let exe = shadow_copy_for_windows(exe)?;
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

/// On Windows, copy the executable next to itself (`app.exe` ->
/// `app.topcoat-dev.exe`) and return the copy's path; other platforms return
/// the path unchanged. The previous server has already been stopped when this
/// runs, but Windows can hold the old image briefly after the process is
/// reaped, so the copy is retried for a moment before giving up.
fn shadow_copy_for_windows(exe: &Path) -> io::Result<PathBuf> {
    if !cfg!(windows) {
        return Ok(exe.to_path_buf());
    }
    let mut file_name = exe.file_stem().unwrap_or_default().to_os_string();
    file_name.push(".topcoat-dev");
    if let Some(extension) = exe.extension() {
        file_name.push(".");
        file_name.push(extension);
    }
    let shadow = exe.with_file_name(file_name);

    let mut last_error = None;
    for _ in 0..20 {
        match std::fs::copy(exe, &shadow) {
            Ok(_) => return Ok(shadow),
            Err(error) => last_error = Some(error),
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(last_error.expect("copy attempted at least once"))
}
