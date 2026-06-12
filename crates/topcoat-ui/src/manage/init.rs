use std::path::PathBuf;

use super::project::Project;
use super::state::{InstallState, STATE_FILE};

/// The result of [`init`].
pub struct Initialized {
    /// The project-relative path of the created install-state file.
    pub state_file: PathBuf,
    /// The base directory recorded for component install output.
    pub base_dir: PathBuf,
}

/// Sets up a project's initial install state, which the other commands (`add`,
/// `remove`, `list`) require before they will run.
///
/// `base_dir` overrides where components are installed (default
/// `src/components`); it is recorded in the install state so that every registry
/// added later installs under `<base_dir>/<registry-name>`. Errors if the
/// project is already initialized rather than clobbering its state.
pub fn init(project: &Project, base_dir: Option<PathBuf>) -> Result<Initialized, String> {
    let state = InstallState::create(project, base_dir)?;
    Ok(Initialized {
        state_file: PathBuf::from(STATE_FILE),
        base_dir: state.base_dir,
    })
}
