//! Terminal prompts shared by CLI commands.

use std::io::{self, IsTerminal};

/// Whether questions can be asked: both stdin and stderr are terminals.
pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stderr().is_terminal()
}

/// The error returned when a prompt is cancelled with `Esc` or `Ctrl-C`. The prompt has
/// already told the user, so commands exit without printing it again.
pub const CANCELLED: &str = "cancelled";

/// Describes an error returned by a prompt, using [`CANCELLED`] for cancellation.
pub fn error(error: &io::Error) -> String {
    if error.kind() == io::ErrorKind::Interrupted {
        CANCELLED.to_string()
    } else {
        format!("failed to read input: {error}")
    }
}
