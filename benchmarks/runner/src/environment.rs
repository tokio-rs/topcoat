//! The machine and toolchains a run happened on.

use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Stdio},
};

use serde::{Deserialize, Serialize};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use crate::manifest::{Manifest, find_on_path};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Environment {
    pub os: Option<String>,
    pub os_version: Option<String>,
    pub kernel: Option<String>,
    pub arch: String,
    pub cpu: Option<String>,
    pub physical_cores: Option<usize>,
    pub logical_cores: usize,
    pub memory_bytes: u64,
    /// `pss` (Linux) or `rss`: the per-process memory metric summed over each app's tree.
    pub memory_metric: String,
    pub git_commit: Option<String>,
    /// `true` if the working tree had uncommitted changes.
    pub git_dirty: Option<bool>,
    pub oha: Option<String>,
    /// `true` when run inside a Nix shell.
    pub nix_shell: bool,
    /// First line of `<command> --version` for each command the selected apps require.
    pub toolchains: BTreeMap<String, String>,
    /// Tools from `expected_tools` whose version differs, as `command -> description`.
    #[serde(default)]
    pub tool_version_mismatches: BTreeMap<String, String>,
}

impl Environment {
    pub fn collect(root: &Path, apps: &[&Manifest], expected: &BTreeMap<String, String>) -> Self {
        let system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::nothing())
                .with_memory(MemoryRefreshKind::nothing().with_ram()),
        )
        .ok();
        let mut commands: Vec<&str> = vec!["rustc", "cargo"];
        for app in apps {
            commands.extend(app.requires.iter().map(String::as_str));
        }
        commands.extend(expected.keys().map(String::as_str));
        commands.sort_unstable();
        commands.dedup();
        let toolchains = commands
            .into_iter()
            .filter(|command| find_on_path(command).is_some())
            .filter_map(|command| Some((command.to_owned(), tool_version(command)?)))
            .collect();
        let git_dirty =
            output("git", &["status", "--porcelain"], root).map(|text| !text.is_empty());
        Self {
            os: System::name().ok(),
            os_version: System::long_os_version().ok(),
            kernel: System::kernel_version().ok(),
            arch: std::env::consts::ARCH.to_owned(),
            cpu: system.as_ref().and_then(|system| {
                system
                    .cpus()
                    .first()
                    .map(|cpu| cpu.brand().trim().to_owned())
            }),
            physical_cores: System::physical_core_count().ok(),
            logical_cores: std::thread::available_parallelism().map_or(1, usize::from),
            memory_bytes: system.as_ref().map_or(0, System::total_memory),
            memory_metric: crate::sampler::memory_metric().to_owned(),
            git_commit: output("git", &["rev-parse", "HEAD"], root),
            git_dirty,
            oha: crate::load::oha_version(),
            nix_shell: std::env::var_os("IN_NIX_SHELL").is_some(),
            tool_version_mismatches: version_mismatches(expected)
                .into_iter()
                .map(|mismatch| (mismatch.tool.clone(), mismatch.to_string()))
                .collect(),
            toolchains,
        }
    }
}

/// The version line of a command, preferring a line that names the command.
pub fn tool_version(command: &str) -> Option<String> {
    let args: &[&str] = match command {
        "go" | "templ" => &["version"],
        // The Tailwind CSS v4 CLI has no `--version`; its help starts with the version.
        "tailwindcss" => &["--help"],
        _ => &["--version"],
    };
    let text = output(command, args, Path::new("."))?;
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let needle = command.to_ascii_lowercase();
    let line = lines
        .iter()
        .find(|line| line.to_ascii_lowercase().contains(&needle))
        .or_else(|| lines.first())?;
    // Drop decoration such as a leading symbol or comment markers around the line.
    let line = line
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .trim_end_matches(|c: char| c == '*' || c == '/' || c.is_whitespace());
    Some(line.to_owned())
}

fn output(command: &str, args: &[&str], dir: &Path) -> Option<String> {
    let output = Command::new(command)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if text.is_empty() {
        String::from_utf8_lossy(&output.stderr)
            .trim()
            .clone_into(&mut text);
    }
    Some(text)
}

/// A tool on `PATH` whose version differs from `expected_tools`.
pub struct Mismatch {
    pub tool: String,
    pub expected: String,
    pub found: String,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "expected {}, found `{}`", self.expected, self.found)
    }
}

/// Tools from `expected` that are on `PATH` but report another version. Tools that are not on
/// `PATH` are not listed; apps that need them are skipped anyway.
pub fn version_mismatches(expected: &BTreeMap<String, String>) -> Vec<Mismatch> {
    expected
        .iter()
        .filter(|(tool, _)| find_on_path(tool).is_some())
        .filter_map(|(tool, version)| {
            let found = tool_version(tool).unwrap_or_else(|| "no version output".to_owned());
            (!reports_version(&found, version)).then(|| Mismatch {
                tool: tool.clone(),
                expected: version.clone(),
                found,
            })
        })
        .collect()
}

/// Returns `true` if a version line such as `tailwindcss v4.3.3` names exactly `version`.
fn reports_version(line: &str, version: &str) -> bool {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '+'))
        .map(|token| token.strip_prefix('v').unwrap_or(token))
        .any(|token| token == version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_lines_must_name_the_exact_version() {
        assert!(reports_version("tailwindcss v4.3.3", "4.3.3"));
        assert!(reports_version(
            "go version go1.26.8 darwin/arm64",
            "go1.26.8"
        ));
        assert!(!reports_version("tailwindcss v4.3.30", "4.3.3"));
        assert!(!reports_version("tailwindcss v4.3.2", "4.3.3"));
    }
}
