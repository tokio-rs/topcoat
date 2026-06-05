use std::path::Path;

use clap::Args;
use console::style;
use topcoat_ui::{Registry, Source};

use super::module;
use super::state::{InstallState, InstalledComponent, STATE_FILE};

#[derive(Args)]
pub(super) struct AddCommand {
    /// Name of the component to add (e.g. `button`)
    component: String,
    /// Named registry to add from
    #[arg(short, long, default_value = "default")]
    registry: String,
    /// Registry location (a path, `file://` path, or `http(s)://` URL); sets or
    /// overrides the location stored for the registry
    #[arg(short, long)]
    url: Option<String>,
    /// Overwrite the component file if it already exists
    #[arg(short, long)]
    force: bool,
}

impl AddCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner().await {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    async fn run_inner(self) -> Result<(), String> {
        let state_path = Path::new(STATE_FILE);
        let mut state = InstallState::load(state_path)?;

        // Resolve (and, with --url, create or update) the named registry. The
        // location is sticky: it stays recorded for later commands.
        let registry_state = state.registry_mut(&self.registry, self.url)?;
        let location = registry_state.url.clone();
        let components_dir = registry_state.components_dir.clone();

        let registry = Registry::load(Source::parse(&location))
            .await
            .map_err(|error| format!("failed to load registry {location}: {error}"))?;

        let component = registry.get(&self.component).ok_or_else(|| {
            let available: Vec<&str> = registry.names().collect();
            format!(
                "unknown component `{}` in registry `{}`; available: {}",
                self.component,
                self.registry,
                available.join(", ")
            )
        })?;

        std::fs::create_dir_all(&components_dir)
            .map_err(|error| format!("failed to create {}: {error}", components_dir.display()))?;

        let file = components_dir.join(component.file_name());
        if file.exists() && !self.force {
            return Err(format!(
                "{} already exists; pass --force to overwrite",
                file.display()
            ));
        }

        let source = component
            .fetch_source()
            .await
            .map_err(|error| format!("failed to read component `{}`: {error}", component.name()))?;
        std::fs::write(&file, source)
            .map_err(|error| format!("failed to write {}: {error}", file.display()))?;

        module::declare(&components_dir, component.file_name())?;

        state
            .registries
            .get_mut(&self.registry)
            .expect("registry resolved above")
            .components
            .insert(
                component.name().to_string(),
                InstalledComponent {
                    version: component.version().to_string(),
                    file: file.clone(),
                },
            );
        state.save(state_path)?;

        println!(
            "{} added {} from {}",
            style("✓").green(),
            style(file.display()).bold(),
            self.registry
        );
        Ok(())
    }
}
