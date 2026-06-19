//! Project-side management of installed components.
//!
//! Where [`crate::registry`] models a registry of available components, this
//! module manages a *project's* relationship to those registries: which
//! components it has installed, from which registry, and at which hash. It
//! backs the `topcoat ui` CLI commands but holds no terminal or presentation
//! logic: commands are exposed as functions ([`init`], [`add`], [`list`],
//! [`remove`]) that return structured results, and any interactive decision is
//! delegated to a caller-supplied [`Confirm`] callback.
//!
//! A project must be set up with [`init`] before the other commands will run;
//! [`add`], [`remove`], and [`list`] error until an install state exists.

mod add;
mod init;
mod list;
mod module;
mod project;
mod remove;
mod state;
mod workspace;

pub use add::{AddOptions, AddOutcome, AddedComponent, add};
pub use init::{InitOptions, Initialized, InstalledThemeInfo, init};
pub use list::{ComponentStatus, InstallStatus, RegistryListing, list};
pub use project::Project;
pub use remove::{Removed, remove};

/// A callback the caller supplies to answer a yes/no question, e.g. whether to
/// pull a component from a non-default registry, or replace a file provided by
/// another registry. Returning `Err` aborts the operation.
pub type Confirm<'a> = dyn FnMut(&str) -> Result<bool, String> + 'a;

/// A callback the caller supplies to pick a theme during [`init`] when one was
/// not named explicitly. Given the available theme names, it returns the chosen
/// one. A theme is mandatory, so there is no way to decline. Returning `Err`
/// aborts init.
pub type ChooseTheme<'a> = dyn FnMut(&[String]) -> Result<String, String> + 'a;
