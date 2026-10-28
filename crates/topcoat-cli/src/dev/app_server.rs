use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{ExitStatus, Stdio},
};

use tokio::process::{Child, Command};
use topcoat_asset::AssetBundle;

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
            Ok(_) => {
                copy_bundle_for_shadow(exe, &shadow)?;
                return Ok(shadow);
            }
            Err(error) => last_error = Some(error),
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    Err(last_error.expect("copy attempted at least once"))
}

/// Copies the executable-scoped bundle beside the Windows shadow executable.
///
/// `AssetBundle::load` derives its default bundle path from the running executable,
/// whose name includes the `.topcoat-dev` suffix while `topcoat dev` is running.
fn copy_bundle_for_shadow(exe: &Path, shadow: &Path) -> io::Result<()> {
    let source = AssetBundle::dir_for_artifact(exe)?;
    match fs::metadata(&source) {
        Ok(metadata) if metadata.is_dir() => {
            let destination = AssetBundle::dir_for_artifact(shadow)?;
            copy_directory(&source, &destination)
        }
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("asset bundle path is not a directory: {}", source.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let destination = AssetBundle::dir_for_artifact(shadow)?;
            remove_directory_if_exists(&destination)
        }
        Err(error) => Err(error),
    }
}

fn copy_directory(source: &Path, destination: &Path) -> io::Result<()> {
    remove_directory_if_exists(destination)?;
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_directory(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "unsupported file in asset bundle: {}",
                    entry.path().display()
                ),
            ));
        }
    }
    Ok(())
}

fn remove_directory_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn copies_executable_scoped_bundle_for_shadow_process() {
        let root =
            std::env::temp_dir().join(format!("topcoat-app-server-shadow-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let executable = root.join("app.exe");
        let shadow = root.join("app.topcoat-dev.exe");
        let source = AssetBundle::dir_for_artifact(&executable).unwrap();
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("manifest.toml"), "bundle manifest").unwrap();
        fs::create_dir_all(source.join("nested")).unwrap();
        fs::write(source.join("nested").join("asset.txt"), "asset data").unwrap();
        let destination = AssetBundle::dir_for_artifact(&shadow).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("stale-hash.txt"), "stale asset").unwrap();

        copy_bundle_for_shadow(&executable, &shadow).unwrap();

        assert_eq!(
            fs::read_to_string(destination.join("manifest.toml")).unwrap(),
            "bundle manifest"
        );
        assert_eq!(
            fs::read_to_string(destination.join("nested").join("asset.txt")).unwrap(),
            "asset data"
        );
        assert!(!destination.join("stale-hash.txt").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn removes_stale_shadow_bundle_when_source_bundle_is_missing() {
        let root = std::env::temp_dir().join(format!(
            "topcoat-app-server-missing-bundle-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let executable = root.join("app.exe");
        let shadow = root.join("app.topcoat-dev.exe");
        let destination = AssetBundle::dir_for_artifact(&shadow).unwrap();
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("manifest.toml"), "stale bundle").unwrap();

        copy_bundle_for_shadow(&executable, &shadow).unwrap();

        assert!(!destination.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
