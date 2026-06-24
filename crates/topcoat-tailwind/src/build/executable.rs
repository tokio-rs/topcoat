use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::build::{BuildError, Result};

const REPO: &str = "tailwindlabs/tailwindcss";

/// Returns the GitHub release asset name for the host platform.
fn asset_name() -> Result<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    Ok(match (os, arch) {
        ("macos", "x86_64") => "tailwindcss-macos-x64",
        ("macos", "aarch64") => "tailwindcss-macos-arm64",
        ("linux", "x86_64") => "tailwindcss-linux-x64",
        ("linux", "aarch64") => "tailwindcss-linux-arm64",
        ("linux", "arm") => "tailwindcss-linux-armv7",
        ("windows", "x86_64") => "tailwindcss-windows-x64.exe",
        ("windows", "aarch64") => "tailwindcss-windows-arm64.exe",
        _ => return Err(BuildError::UnsupportedPlatform { os, arch }),
    })
}

/// Download the Tailwind CLI for `version` to `dest`.
///
/// If `dest` already exists it's left untouched. On Unix the file is made
/// executable.
///
/// # Errors
///
/// Returns `Err` if the host platform is unsupported, if the download request
/// or body read fails, or if creating, writing, chmod-ing, or renaming the
/// destination file fails.
pub fn download(version: &str, dest: impl AsRef<Path>) -> Result<PathBuf> {
    let dest = dest.as_ref().to_path_buf();
    if dest.exists() {
        return Ok(dest);
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|source| BuildError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    let url = format!(
        "https://github.com/{REPO}/releases/download/v{version}/{name}",
        name = asset_name()?,
    );

    let mut body = ureq::get(&url)
        .call()
        .map_err(|e| BuildError::Http(Box::new(e)))?
        .into_body();
    let mut reader = body.as_reader();

    let temp = temp_path(&dest);
    let mut file = fs::File::create(&temp).map_err(|source| BuildError::Io {
        path: temp.clone(),
        source,
    })?;
    io::copy(&mut reader, &mut file).map_err(|source| BuildError::Io {
        path: temp.clone(),
        source,
    })?;
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp)
            .map_err(|source| BuildError::Io {
                path: temp.clone(),
                source,
            })?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp, perms).map_err(|source| BuildError::Io {
            path: temp.clone(),
            source,
        })?;
    }

    fs::rename(&temp, &dest).map_err(|source| BuildError::Io {
        path: dest.clone(),
        source,
    })?;

    Ok(dest)
}

fn temp_path(dest: &Path) -> PathBuf {
    let file_name = dest
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .unwrap_or("tailwindcss");
    dest.with_file_name(format!("{file_name}.tmp"))
}
