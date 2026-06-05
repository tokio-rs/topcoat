use std::io::ErrorKind;

use clap::Args;
use console::style;

use super::module;
use super::project::{Project, ProjectArg};
use super::state::InstallState;

#[derive(Args)]
pub(super) struct RemoveCommand {
    /// Name of the component to remove
    component: String,
    /// Registry the component was added from (searched across all if omitted)
    #[arg(short, long)]
    registry: Option<String>,
    #[command(flatten)]
    project: ProjectArg,
}

impl RemoveCommand {
    pub(super) async fn run(self) {
        if let Err(error) = self.run_inner().await {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    async fn run_inner(self) -> Result<(), String> {
        let project = Project::locate(self.project).await?;
        let mut state = InstallState::load(&project)?;

        let registry_name = resolve_registry(&self.component, self.registry.as_deref(), &state)?;

        let registry = state
            .registries
            .get_mut(&registry_name)
            .expect("registry resolved above");
        let installed = registry
            .components
            .remove(&self.component)
            .expect("component resolved above");
        let components_dir = registry.components_dir.clone();

        let file = project.resolve(&installed.file);
        match std::fs::remove_file(&file) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("failed to remove {}: {error}", file.display()));
            }
        }

        if let Some(file_name) = installed.file.file_name().and_then(|name| name.to_str()) {
            module::undeclare(&project.resolve(&components_dir), file_name)?;
        }

        state.save(&project)?;

        println!(
            "{} removed {} from {}",
            style("✓").green(),
            style(installed.file.display()).bold(),
            registry_name
        );
        Ok(())
    }
}

/// Determines which registry the component should be removed from: the one
/// named via `--registry`, or the sole registry that has it installed.
fn resolve_registry(
    component: &str,
    registry: Option<&str>,
    state: &InstallState,
) -> Result<String, String> {
    if let Some(name) = registry {
        let registry = state
            .registries
            .get(name)
            .ok_or_else(|| format!("unknown registry `{name}`"))?;
        if !registry.components.contains_key(component) {
            return Err(format!(
                "component `{component}` is not installed from registry `{name}`"
            ));
        }
        return Ok(name.to_string());
    }

    let matches: Vec<&String> = state
        .registries
        .iter()
        .filter(|(_, registry)| registry.components.contains_key(component))
        .map(|(name, _)| name)
        .collect();

    match matches.as_slice() {
        [] => Err(format!("component `{component}` is not installed")),
        [name] => Ok((*name).clone()),
        many => Err(format!(
            "component `{component}` is installed from multiple registries ({}); pass --registry to choose",
            many.iter().map(|name| name.as_str()).collect::<Vec<_>>().join(", ")
        )),
    }
}
