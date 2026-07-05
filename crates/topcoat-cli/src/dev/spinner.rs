use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

/// A single-line terminal spinner shown while a long-running step is in
/// flight.
///
/// The spinner ticks on a background thread, so it animates even while the
/// owning task is busy. Dropping it clears the line it occupied.
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
