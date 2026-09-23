use std::{
    io,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
};

use tokio::process::{Child, Command};

use super::port::Address;

/// A running development application.
///
/// Inherits terminal input and output. Receives the dev server URL through
/// `TOPCOAT_DEV_URL` for readiness notifications and page updates.
pub struct AppServer {
    child: Child,
}

impl AppServer {
    /// Starts the built executable. On Windows, runs a copy so the original file
    /// remains available for rebuilding.
    pub fn spawn(exe: &Path, dev_url: &str, address: &Address) -> io::Result<Self> {
        let exe = shadow_copy_for_windows(exe)?;
        let child = Command::new(exe)
            .env("TOPCOAT_DEV_URL", dev_url)
            .env("HOST", &address.host)
            .env("PORT", address.port.to_string())
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

/// Copies the executable to a separate path on Windows so a running process does not
/// lock the build output. Retries briefly while Windows releases the previous copy.
/// Other platforms use the original path.
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
