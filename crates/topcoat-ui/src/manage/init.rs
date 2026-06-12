use std::path::PathBuf;

use super::project::Project;
use super::state::{InstallState, STATE_FILE};

/// How to set up a project's install state.
pub struct InitOptions {
    /// Base directory for component install output (default `src/components`).
    pub base_dir: Option<PathBuf>,
    /// Location of the initial default registry (a path, `file://` path, or
    /// `http(s)://` URL). When omitted, the built-in `topcoat` registry is used.
    pub url: Option<String>,
}

/// The result of [`init`].
pub struct Initialized {
    /// The project-relative path of the created install-state file.
    pub state_file: PathBuf,
    /// The name of the default registry seeded into the install state.
    pub registry: String,
    /// The base directory recorded for component install output.
    pub base_dir: PathBuf,
}

/// Sets up a project's initial install state, which the other commands (`add`,
/// `remove`, `list`) require before they will run.
///
/// The state is seeded with the initial default registry — the built-in
/// `topcoat`, or the registry at `options.url` — so every later command has a
/// registry to work against. The recorded base directory determines where each
/// registry, this one and any added later, installs its components. Errors if
/// the project is already initialized rather than clobbering its state.
pub async fn init(project: &Project, options: InitOptions) -> Result<Initialized, String> {
    let state = InstallState::create(project, options.base_dir, options.url).await?;
    Ok(Initialized {
        state_file: PathBuf::from(STATE_FILE),
        registry: state
            .default_registry
            .expect("init seeds a default registry"),
        base_dir: state.base_dir,
    })
}
