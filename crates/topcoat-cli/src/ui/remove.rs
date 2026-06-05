use std::io::ErrorKind;
use std::path::Path;

use clap::Args;
use console::style;

use super::module;
use super::state::{InstallState, STATE_FILE};

#[derive(Args)]
pub(super) struct RemoveCommand {
    /// Name of the component to remove
    component: String,
    /// Registry the component was added from (searched across all if omitted)
    #[arg(short, long)]
    registry: Option<String>,
}

impl RemoveCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let state_path = Path::new(STATE_FILE);
        let mut state = InstallState::load(state_path)?;

        let registry_name = self.resolve_registry(&state)?;

        let registry = state
            .registries
            .get_mut(&registry_name)
            .expect("registry resolved above");
        let installed = registry
            .components
            .remove(&self.component)
            .expect("component resolved above");
        let components_dir = registry.components_dir.clone();

        match std::fs::remove_file(&installed.file) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("failed to remove {}: {error}", installed.file.display()));
            }
        }

        if let Some(file_name) = installed.file.file_name().and_then(|name| name.to_str()) {
            module::undeclare(&components_dir, file_name)?;
        }

        state.save(state_path)?;

        println!(
            "{} removed {} from {}",
            style("✓").green(),
            style(installed.file.display()).bold(),
            registry_name
        );
        Ok(())
    }

    /// Determines which registry the component should be removed from: the one
    /// named via `--registry`, or the sole registry that has it installed.
    fn resolve_registry(&self, state: &InstallState) -> Result<String, String> {
        if let Some(name) = &self.registry {
            let registry = state
                .registries
                .get(name)
                .ok_or_else(|| format!("unknown registry `{name}`"))?;
            if !registry.components.contains_key(&self.component) {
                return Err(format!(
                    "component `{}` is not installed from registry `{name}`",
                    self.component
                ));
            }
            return Ok(name.clone());
        }

        let matches: Vec<&String> = state
            .registries
            .iter()
            .filter(|(_, registry)| registry.components.contains_key(&self.component))
            .map(|(name, _)| name)
            .collect();

        match matches.as_slice() {
            [] => Err(format!("component `{}` is not installed", self.component)),
            [name] => Ok((*name).clone()),
            many => Err(format!(
                "component `{}` is installed from multiple registries ({}); pass --registry to choose",
                self.component,
                many.iter().map(|name| name.as_str()).collect::<Vec<_>>().join(", ")
            )),
        }
    }
}
