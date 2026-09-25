//! Installs and manages components in a Cargo package.
//!
//! Operations return structured results. Supply callbacks to answer
//! confirmation prompts and choose a theme.
//!
//! Call [`init`] to create the package's install state before managing its
//! components.

mod add;
mod init;
mod list;
mod module;
mod package;
mod remove;
mod state;
mod workspace;

pub use add::*;
pub use init::*;
pub use list::*;
pub use package::*;
pub use remove::*;

/// Answers a confirmation prompt. Return `true` to accept, `false` to decline,
/// or `Err` to abort the operation with an error.
pub type Confirm<'a> = dyn FnMut(&str) -> Result<bool, String> + 'a;

/// Chooses a theme during initialization. Return one of the supplied names,
/// or `Err` to abort initialization.
pub type ChooseTheme<'a> = dyn FnMut(&[String]) -> Result<String, String> + 'a;
