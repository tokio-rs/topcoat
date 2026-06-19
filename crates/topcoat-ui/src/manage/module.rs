use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// The module name a component file is declared under, or `None` if the file is
/// not an ordinary `*.rs` module (e.g. `mod.rs` itself).
fn module_name(file_name: &str) -> Option<&str> {
    match file_name.strip_suffix(".rs") {
        Some("mod") | None => None,
        Some(module) => Some(module),
    }
}

/// The file that holds the components directory's module declarations.
///
/// By default this is a sibling `<dir>.rs` file next to the directory. If the
/// user instead keeps a `mod.rs` inside the directory, that file is maintained,
/// so either Rust module style works.
///
/// Rust forbids declaring a module via both files at once, so when both exist
/// this errors rather than silently picking one (which would leave a stale file
/// and a package that does not compile).
fn module_file(dir: &Path) -> Result<PathBuf, String> {
    let mod_path = dir.join("mod.rs");

    let sibling = dir.file_name().map(|name| {
        let mut sibling = name.to_os_string();
        sibling.push(".rs");
        // A directory without a final component (e.g. the filesystem root) has no
        // sibling to name; `None` falls back to `mod.rs` below.
        dir.parent().unwrap_or(dir).join(sibling)
    });

    match sibling {
        Some(sibling) if mod_path.exists() && sibling.exists() => Err(format!(
            "components module declared at both {} and {}; delete one to choose a module style",
            sibling.display(),
            mod_path.display(),
        )),
        _ if mod_path.exists() => Ok(mod_path),
        Some(sibling) => Ok(sibling),
        None => Ok(mod_path),
    }
}

/// Verifies the components directory does not declare its module via both a
/// sibling `<dir>.rs` and an inner `mod.rs`. Call this before mutating any files
/// so an ambiguous layout aborts before components are written or removed.
pub(super) fn check(dir: &Path) -> Result<(), String> {
    module_file(dir).map(|_| ())
}

/// Declares the component file in the components directory's module file so it is
/// reachable, creating or appending to the file as needed.
pub(super) fn declare(dir: &Path, file_name: &str) -> Result<(), String> {
    let Some(module) = module_name(file_name) else {
        return Ok(());
    };

    let mod_path = module_file(dir)?;
    let declaration = format!("pub mod {module};");

    let mut contents = match std::fs::read_to_string(&mod_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("failed to read {}: {error}", mod_path.display())),
    };

    if contents.lines().any(|line| line.trim() == declaration) {
        return Ok(());
    }
    if !contents.is_empty() && !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(&declaration);
    contents.push('\n');

    std::fs::write(&mod_path, contents)
        .map_err(|error| format!("failed to write {}: {error}", mod_path.display()))
}

/// Removes the component file's declaration from the components directory's
/// module file. The file is deleted entirely once it holds no more declarations.
pub(super) fn undeclare(dir: &Path, file_name: &str) -> Result<(), String> {
    let Some(module) = module_name(file_name) else {
        return Ok(());
    };

    let mod_path = module_file(dir)?;
    let declaration = format!("pub mod {module};");

    let contents = match std::fs::read_to_string(&mod_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("failed to read {}: {error}", mod_path.display())),
    };

    let kept: Vec<&str> = contents
        .lines()
        .filter(|line| line.trim() != declaration)
        .collect();

    if kept.iter().all(|line| line.trim().is_empty()) {
        return match std::fs::remove_file(&mod_path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!("failed to remove {}: {error}", mod_path.display())),
        };
    }

    let mut updated = kept.join("\n");
    updated.push('\n');
    std::fs::write(&mod_path, updated)
        .map_err(|error| format!("failed to write {}: {error}", mod_path.display()))
}
