use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

/// A terminal spinner for an operation in progress. A background thread animates it,
/// and dropping it clears its line.
pub struct Spinner(ProgressBar);

impl Spinner {
    pub fn new(message: &str) -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("  {spinner:.cyan} {msg}")
                .unwrap(),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(80));
        Self(spinner)
    }

    /// A cloneable handle to the underlying [`ProgressBar`], for updating the
    /// message from another task or callback.
    pub fn bar(&self) -> ProgressBar {
        self.0.clone()
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.0.finish_and_clear();
    }
}
