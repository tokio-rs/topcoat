use console::strip_ansi_codes;

/// How much of the stream to keep for reporting a failure.
///
/// Cargo redraws its progress bar in place, so most of a long build's stderr
/// is renders that were immediately overwritten. Keeping only the tail bounds
/// that without losing the error report, which cargo prints last.
const CAPTURE_LIMIT: usize = 256 * 1024;

/// The tail of cargo's stderr, captured while a build runs.
#[derive(Default)]
pub(super) struct StderrTail {
    captured: Vec<u8>,
}

impl StderrTail {
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Append the next chunk of the stream, trimming the front once the
    /// capture grows well past [`CAPTURE_LIMIT`].
    pub(super) fn push(&mut self, chunk: &[u8]) {
        self.captured.extend_from_slice(chunk);
        if self.captured.len() > CAPTURE_LIMIT * 2 {
            let drain_to = self.captured.len() - CAPTURE_LIMIT;
            self.captured.drain(..drain_to);
        }
    }

    /// Cargo's own error report, extracted from the stderr it interleaves
    /// with status lines and progress bar renders.
    ///
    /// Cargo redraws the progress bar in place with carriage returns rather
    /// than newlines, so only the text after the last `\r` of a line was ever
    /// visible; the report itself runs from the first `error` line to the end
    /// of the stream. When there is no such line the build died without
    /// reporting anything (killed by a signal, say), and the status lines are
    /// all there is to go on.
    pub(super) fn error_output(&self) -> String {
        let stderr = String::from_utf8_lossy(&self.captured);
        let lines: Vec<String> = stderr
            .split('\n')
            .map(|line| {
                let visible = line.rsplit('\r').next().unwrap_or_default();
                strip_ansi_codes(visible).trim_end().to_string()
            })
            .collect();

        let report = match lines.iter().position(|line| line.starts_with("error")) {
            Some(start) => lines[start..].join("\n"),
            None => lines
                .iter()
                .filter(|line| !line.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join("\n"),
        };
        report.trim_end().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tail(stderr: &str) -> StderrTail {
        let mut tail = StderrTail::new();
        tail.push(stderr.as_bytes());
        tail
    }

    #[test]
    fn progress_renders_and_color_are_stripped_from_cargo_output() {
        let stderr = "    Building [==>  ] 1/2: app  \r\u{1b}[1m\u{1b}[31merror\u{1b}[0m: boom\n";
        assert_eq!(tail(stderr).error_output(), "error: boom");
    }

    #[test]
    fn a_build_that_reports_no_error_falls_back_to_status_lines() {
        // Nothing to key off when cargo is killed outright, so report what it
        // managed to say rather than nothing at all.
        let stderr = "   Compiling app v0.1.0 (/app)\n";
        assert_eq!(
            tail(stderr).error_output(),
            "   Compiling app v0.1.0 (/app)"
        );
    }
}
