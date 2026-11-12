//! The self-contained HTML report.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::results::Results;

/// The report page. `/*BENCH_DATA*/null` is replaced with the results.
const TEMPLATE: &str = include_str!("report/template.html");
const PLACEHOLDER: &str = "/*BENCH_DATA*/null";

/// Writes the report for `results` to `path`.
pub fn write(results: &Results, path: &Path) -> Result<()> {
    let json = serde_json::to_string(results)?;
    // Keep the data from closing the script element or opening a comment.
    let json = json.replace("</", "<\\/").replace("<!--", "<\\!--");
    let html = TEMPLATE.replacen(PLACEHOLDER, &json, 1);
    std::fs::write(path, html).with_context(|| format!("cannot write {}", path.display()))
}

/// Regenerates `report.html` next to the `results.json` at `path` (a file or its directory)
/// and returns the directory.
pub fn regenerate(path: &Path) -> Result<PathBuf> {
    let (dir, file) = if path.is_dir() {
        (path.to_path_buf(), path.join("results.json"))
    } else {
        let dir = path
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        (dir, path.to_path_buf())
    };
    let results = Results::load(&file)?;
    let report = dir.join("report.html");
    write(&results, &report)?;
    println!("Report: {}", report.display());
    Ok(dir)
}

/// Opens `path` in the default browser.
pub fn open(path: &Path) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    if let Err(error) = std::process::Command::new(opener).arg(path).status() {
        crate::run::warn(&format!("cannot open the report with `{opener}`: {error}"));
    }
}
