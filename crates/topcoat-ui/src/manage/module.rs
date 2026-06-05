use std::io::ErrorKind;
use std::path::Path;

/// The module name a component file is declared under, or `None` if the file is
/// not an ordinary `*.rs` module (e.g. `mod.rs` itself).
fn module_name(file_name: &str) -> Option<&str> {
    match file_name.strip_suffix(".rs") {
        Some("mod") | None => None,
        Some(module) => Some(module),
    }
}

/// Declares the component file in the components directory's `mod.rs` so it is
/// reachable, creating or appending to the file as needed.
pub(super) fn declare(dir: &Path, file_name: &str) -> Result<(), String> {
    let Some(module) = module_name(file_name) else {
        return Ok(());
    };

    let mod_path = dir.join("mod.rs");
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

    std::fs::write(&mod_path, contents).map_err(|error| format!("failed to write {}: {error}", mod_path.display()))
}

/// Removes the component file's declaration from the components directory's
/// `mod.rs`. The file is deleted entirely once it holds no more declarations.
pub(super) fn undeclare(dir: &Path, file_name: &str) -> Result<(), String> {
    let Some(module) = module_name(file_name) else {
        return Ok(());
    };

    let mod_path = dir.join("mod.rs");
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
    std::fs::write(&mod_path, updated).map_err(|error| format!("failed to write {}: {error}", mod_path.display()))
}
