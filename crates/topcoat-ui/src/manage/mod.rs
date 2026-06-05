//! Project-side management of installed components.
//!
//! Where [`crate::registry`] models a registry of available components, this
//! module manages a *project's* relationship to those registries: which
//! components it has installed, from which registry, and at which version. It
//! backs the `topcoat ui` CLI commands but holds no terminal or presentation
//! logic — commands are exposed as functions ([`add`], [`list`], [`remove`])
//! that return structured results, and any interactive decision is delegated to
//! a caller-supplied [`Confirm`] callback.

mod add;
mod list;
mod module;
mod project;
mod remove;
mod state;

pub use add::{AddOptions, AddOutcome, AddedComponent, add};
pub use list::{ComponentStatus, InstallStatus, RegistryListing, list};
pub use project::Project;
pub use remove::{Removed, remove};

/// A callback the caller supplies to answer a yes/no question — e.g. whether to
/// pull a component from a non-default registry, or replace a file provided by
/// another registry. Returning `Err` aborts the operation.
pub type Confirm<'a> = dyn FnMut(&str) -> Result<bool, String> + 'a;
