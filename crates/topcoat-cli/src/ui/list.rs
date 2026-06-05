use std::path::Path;

use clap::Args;
use console::style;
use topcoat_ui::{Registry, Source};

use super::state::{InstallState, RegistryState, STATE_FILE};

#[derive(Args)]
pub(super) struct ListCommand {
    /// Limit the listing to a single named registry
    #[arg(short, long)]
    registry: Option<String>,
    /// Only show components that are installed
    #[arg(short, long)]
    installed: bool,
}

impl ListCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner().await {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    async fn run_inner(self) -> Result<(), String> {
        let mut state = InstallState::load(Path::new(STATE_FILE))?;

        // Ensure there is something to list: a named registry that isn't tracked
        // yet (only valid for one with a built-in location), or the project's
        // default registry when nothing has been added yet.
        match &self.registry {
            Some(name) if !state.registries.contains_key(name) => {
                let url = InstallState::default_url(name)
                    .ok_or_else(|| format!("unknown registry `{name}`"))?;
                state
                    .registries
                    .insert(name.clone(), RegistryState::new(url));
            }
            None if state.registries.is_empty() => {
                let name = state.default_registry.clone();
                let url = InstallState::default_url(&name).ok_or_else(|| {
                    format!("default registry `{name}` has no known location; run `topcoat ui add` first")
                })?;
                state.registries.insert(name, RegistryState::new(url));
            }
            _ => {}
        }

        for (name, registry_state) in &state.registries {
            if self
                .registry
                .as_deref()
                .is_some_and(|selected| selected != name)
            {
                continue;
            }
            // Separate registry blocks with a blank line.
            println!();

            list_registry(name, registry_state, self.installed).await;
        }

        Ok(())
    }
}

/// Lists one registry's components. A component counts as installed only when it
/// is tracked under *this* registry, so the same name installed from a different
/// registry is not treated as installed here.
async fn list_registry(name: &str, state: &RegistryState, installed_only: bool) {
    println!(
        "{} {}",
        style(name).bold(),
        style(format!("({})", state.url)).dim()
    );

    let registry = match Registry::load(Source::parse(&state.url)).await {
        Ok(registry) => registry,
        Err(error) => {
            println!(
                "  {}",
                style(format!("failed to load registry: {error}")).red()
            );
            return;
        }
    };

    let names: Vec<&str> = registry.names().collect();
    for component_name in &names {
        let component = registry
            .get(component_name)
            .expect("name came from the registry");
        let latest = component.version();
        match state.components.get(*component_name) {
            None => {
                if !installed_only {
                    println!("    {component_name} {}", style(latest).dim());
                }
            }
            Some(installed) if installed.version == latest => {
                println!(
                    "  {} {} {} (installed)",
                    style("✓").green(),
                    style(component_name).bold(),
                    style(&installed.version).green()
                );
            }
            Some(installed) => {
                println!(
                    "  {} {} {} → {latest} {}",
                    style("↑").yellow(),
                    style(component_name).bold(),
                    installed.version,
                    style("(update available)").yellow(),
                );
            }
        }
    }

    // Components tracked under this registry that it no longer offers.
    for (component_name, installed) in &state.components {
        if !names.contains(&component_name.as_str()) {
            println!(
                "  {} {component_name} {} {}",
                style("?").red(),
                installed.version,
                style("(not in registry)").red(),
            );
        }
    }
}
