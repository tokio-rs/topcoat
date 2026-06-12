use std::path::PathBuf;

use super::project::Project;
use super::state::{InstallState, STATE_FILE};

/// How to set up a project's install state.
pub struct InitOptions {
    /// Base directory for component install output (default `src/components`).
    pub components_dir: Option<PathBuf>,
}

/// The result of [`init`].
pub struct Initialized {
    /// The project-relative path of the created install-state file.
    pub state_file: PathBuf,
    /// The base directory recorded for component install output.
    pub components_dir: PathBuf,
}

/// Sets up a project's initial install state, which the other commands (`add`,
/// `remove`, `list`) require before they will run.
///
/// Registries are discovered from the project's dependencies, so nothing about
/// them is recorded here; init only fixes where components install. Errors if
/// the project is already initialized rather than clobbering its state.
pub fn init(project: &Project, options: InitOptions) -> Result<Initialized, String> {
    let state = InstallState::create(project, options.components_dir)?;
    Ok(Initialized {
        state_file: PathBuf::from(STATE_FILE),
        components_dir: state.components_dir,
    })
}
