//! Managing the components installed in a package.
//!
//! This module tracks which components a package has installed, from which
//! registry, and at which hash. The state lives in `components.toml` at the
//! package root. Each `topcoat ui` command is a function here: [`init`],
//! [`add`], [`list`], and [`remove`]. They return structured results and do
//! no terminal output. Questions for the user go through callbacks that the
//! caller supplies, [`Confirm`] and [`ChooseTheme`].
//!
//! Run [`init`] on a package first. [`add`], [`list`], and [`remove`] fail
//! until the package has a `components.toml`.

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

/// A callback that answers a yes/no question, given the question as text.
///
/// [`add`] asks, for example, whether to add a component from a registry other
/// than the built-in one, or whether to replace a file that a component from
/// another registry installed. Returning `Err` aborts the operation with that
/// message.
pub type Confirm<'a> = dyn FnMut(&str) -> Result<bool, String> + 'a;

/// A callback that picks a theme during [`init`] when none was named and the
/// registry offers several.
///
/// It receives the available theme names and returns one of them. A theme is
/// required, so the callback cannot skip the choice. Returning `Err` aborts
/// [`init`] with that message.
pub type ChooseTheme<'a> = dyn FnMut(&[String]) -> Result<String, String> + 'a;
