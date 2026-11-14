//! The `list` and `doctor` commands.

use std::{path::Path, process::ExitCode};

use anyhow::Result;
use console::style;

use crate::{
    config::{CliOverrides, Config},
    dataset::Dataset,
    environment::tool_version,
    manifest::{Manifest, find_on_path},
    scenario::Scenario,
};

pub fn list(root: &Path, config_path: &Path) -> Result<()> {
    let (manifests, broken) = Manifest::discover(root)?;
    println!("{}", style("Apps").bold());
    if manifests.is_empty() && broken.is_empty() {
        println!("  none found under {}", root.join("apps").display());
    }
    for app in &manifests {
        println!(
            "  {:<16} {:<22} {:<14} {:<8} {}",
            app.id, app.name, app.rendering, app.language, app.description
        );
    }
    for (id, error) in &broken {
        println!("  {id:<16} {} {error}", style("invalid manifest:").red());
    }
    println!("\n{}", style("Scenarios").bold());
    let dataset = Dataset::load(&root.join("data"));
    for scenario in Scenario::all(&dataset) {
        let path = scenario.paths.first().map_or("", String::as_str);
        let rotation = if scenario.paths.len() > 1 {
            format!(" (+{} more)", scenario.paths.len() - 1)
        } else {
            String::new()
        };
        println!(
            "  {:<11} {:<4} {:<45} {}",
            scenario.id,
            scenario.method,
            format!("{path}{rotation}"),
            style(scenario.description).dim()
        );
    }
    if config_path.exists() {
        let text = crate::read_file(config_path)?;
        let profiles: Vec<String> = toml::from_str::<toml::Table>(&text)?
            .get("profiles")
            .and_then(toml::Value::as_table)
            .map_or_else(Vec::new, |table| table.keys().cloned().collect());
        println!("\n{} {}", style("Profiles").bold(), profiles.join(", "));
    }
    Ok(())
}

pub fn doctor(root: &Path, config_path: &Path) -> ExitCode {
    let mut healthy = true;
    println!("{}", style("Environment").bold());
    let nix = std::env::var_os("IN_NIX_SHELL").is_some();
    println!(
        "  nix shell  {}",
        if nix {
            "yes".to_owned()
        } else {
            "no (run `nix develop .#benchmarks` for pinned toolchains)".to_owned()
        }
    );
    if let Some(version) = crate::load::oha_version() {
        println!("  {} oha        {version}", ok());
    } else {
        healthy = false;
        println!("  {} oha        missing: required to run benchmarks", bad());
    }
    let data = root.join("data").join("products.json");
    if data.is_file() {
        println!("  {} dataset    {}", ok(), data.display());
    } else {
        healthy = false;
        println!("  {} dataset    {} is missing", bad(), data.display());
    }
    let config = match Config::load(config_path, None, &CliOverrides::default()) {
        Ok(config) => {
            println!(
                "  {} config     {} (default profile {})",
                ok(),
                config_path.display(),
                config.profile
            );
            config
        }
        Err(error) => {
            println!("  {} config     {error:#}", bad());
            return ExitCode::FAILURE;
        }
    };
    if let Ok((soft, _)) =
        nix::sys::resource::getrlimit(nix::sys::resource::Resource::RLIMIT_NOFILE)
    {
        println!(
            "  {} open files soft limit {soft} (the runner raises it for oha)",
            ok()
        );
    }

    println!("\n{}", style("Apps").bold());
    let (manifests, broken) = match Manifest::discover(root) {
        Ok(found) => found,
        Err(error) => {
            println!("  {} {error:#}", bad());
            return ExitCode::FAILURE;
        }
    };
    if manifests.is_empty() && broken.is_empty() {
        println!("  none found under {}", root.join("apps").display());
    }
    for (id, error) in &broken {
        healthy = false;
        println!("  {} {id:<16} invalid manifest: {error}", bad());
    }
    let mut checked = std::collections::BTreeMap::new();
    for app in &manifests {
        let missing = app.missing_requirements();
        if missing.is_empty() {
            let version =
                crate::server::framework_version(app, &config).unwrap_or_else(|| "-".to_owned());
            println!("  {} {:<16} ready  {}", ok(), app.id, style(version).dim());
        } else {
            println!("  {} {:<16} missing {}", warn(), app.id, missing.join(", "));
        }
        for command in &app.requires {
            checked.entry(command.clone()).or_insert_with(|| {
                find_on_path(command)
                    .map(|_| tool_version(command).unwrap_or_else(|| "found".to_owned()))
            });
        }
    }
    for tool in config.expected_tools.keys() {
        checked.entry(tool.clone()).or_insert_with(|| {
            find_on_path(tool).map(|_| tool_version(tool).unwrap_or_else(|| "found".to_owned()))
        });
    }
    let mismatches = crate::environment::version_mismatches(&config.expected_tools);
    if !checked.is_empty() {
        println!("\n{}", style("Toolchains").bold());
        for (command, version) in &checked {
            let mismatch = mismatches.iter().find(|m| &m.tool == command);
            match (version, mismatch) {
                (_, Some(mismatch)) => {
                    healthy = false;
                    println!("  {} {command:<14} {mismatch}", bad());
                }
                (Some(version), None) => {
                    println!("  {} {command:<14} {}", ok(), style(version).dim());
                }
                (None, None) => println!("  {} {command:<14} not on PATH", warn()),
            }
        }
    }
    if healthy {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn ok() -> console::StyledObject<&'static str> {
    style("ok").green()
}

fn warn() -> console::StyledObject<&'static str> {
    style("--").yellow()
}

fn bad() -> console::StyledObject<&'static str> {
    style("!!").red().bold()
}
