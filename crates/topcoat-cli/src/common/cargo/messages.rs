use std::path::PathBuf;

use serde::Deserialize;

use super::{artifacts::Artifact, stderr::StderrTail};

/// Diagnostics and artifacts read from Cargo's JSON build output.
pub(super) struct Messages(Vec<Message>);

impl Messages {
    /// Parse cargo's `--message-format=json` stdout, skipping lines that are
    /// not valid messages.
    pub(super) fn parse(stdout: &str) -> Self {
        Self(
            stdout
                .lines()
                .filter_map(|line| serde_json::from_str(line).ok())
                .collect(),
        )
    }

    /// Returns diagnostics for a failed build.
    ///
    /// Uses compiler diagnostics when rustc reported an error. Otherwise, uses Cargo's
    /// stderr, since Cargo failures may not appear in the JSON output.
    pub(super) fn failure_diagnostics(&self, stderr: &StderrTail) -> String {
        let diagnostics = if self.has_compiler_error() {
            self.rendered_diagnostics()
        } else {
            stderr.error_output()
        };
        diagnostics.trim_end().to_string()
    }

    /// Whether rustc reported an error, including an internal compiler error.
    fn has_compiler_error(&self) -> bool {
        self.diagnostics().any(Diagnostic::is_error)
    }

    /// The rendered text of every diagnostic rustc reported, in order.
    /// Warnings are kept alongside the errors, matching what a plain
    /// `cargo build` prints.
    fn rendered_diagnostics(&self) -> String {
        self.diagnostics()
            .filter_map(|diagnostic| diagnostic.rendered.as_deref())
            .collect()
    }

    fn diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.0.iter().filter_map(|message| match message {
            Message::Compiler { message } => Some(message),
            _ => None,
        })
    }

    /// The final linked outputs of the build, from every artifact's
    /// [`Artifact::final_outputs`].
    pub(super) fn artifacts(&self) -> Vec<PathBuf> {
        self.0
            .iter()
            .filter_map(|message| match message {
                Message::Artifact(artifact) => Some(artifact),
                _ => None,
            })
            .flat_map(Artifact::final_outputs)
            .collect()
    }
}

/// A Cargo JSON message. Output unrelated to diagnostics or artifacts is parsed as
/// [`Message::Other`].
#[derive(Deserialize)]
#[serde(tag = "reason")]
enum Message {
    #[serde(rename = "compiler-message")]
    Compiler { message: Diagnostic },
    #[serde(rename = "compiler-artifact")]
    Artifact(Artifact),
    #[serde(other)]
    Other,
}

/// A diagnostic rustc reported, with the text it would have printed to a
/// terminal.
#[derive(Deserialize)]
struct Diagnostic {
    level: String,
    rendered: Option<String>,
}

impl Diagnostic {
    fn is_error(&self) -> bool {
        self.level.starts_with("error")
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn messages(values: &[serde_json::Value]) -> Messages {
        let lines: Vec<String> = values.iter().map(ToString::to_string).collect();
        Messages::parse(&lines.join("\n"))
    }

    fn stderr(text: &str) -> StderrTail {
        let mut tail = StderrTail::new();
        tail.push(text.as_bytes());
        tail
    }

    fn diagnostic(level: &str, rendered: &str) -> serde_json::Value {
        json!({
            "reason": "compiler-message",
            "message": { "level": level, "rendered": rendered },
        })
    }

    #[test]
    fn rustc_errors_are_reported_from_the_json_stream() {
        let messages = messages(&[
            diagnostic("warning", "warning: unused variable `x`\n"),
            diagnostic("error", "error[E0308]: mismatched types\n"),
        ]);
        assert_eq!(
            messages.failure_diagnostics(&stderr(
                "   Compiling app v0.1.0\nerror: could not compile `app` (bin \"app\") due to 1 previous error\n"
            )),
            "warning: unused variable `x`\nerror[E0308]: mismatched types"
        );
    }

    #[test]
    fn build_script_failures_are_reported_from_cargo_stderr() {
        // A build script that exits non-zero is cargo's own failure, not
        // rustc's: stdout carries the build script's artifact and nothing
        // else, so there is no diagnostic to render.
        let messages = messages(&[
            json!({
                "reason": "compiler-artifact",
                "target": { "crate_types": ["bin"] },
                "executable": null,
                "filenames": ["/target/debug/build/app-abc/build-script-build"],
            }),
            json!({ "reason": "build-finished", "success": false }),
        ]);
        let stderr = stderr(concat!(
            "   Compiling app v0.1.0 (/app)\n",
            "    Building [=>     ] 0/3: app(build.rs)  \r",
            "    Building [====>  ] 1/3: app(build)     \r",
            "error: failed to run custom build command for `app v0.1.0 (/app)`\n",
            "\n",
            "Caused by:\n",
            "  process didn't exit successfully: `build-script-build` (exit status: 1)\n",
            "  --- stderr\n",
            "  something went wrong in the build script\n",
        ));
        assert_eq!(
            messages.failure_diagnostics(&stderr),
            concat!(
                "error: failed to run custom build command for `app v0.1.0 (/app)`\n",
                "\n",
                "Caused by:\n",
                "  process didn't exit successfully: `build-script-build` (exit status: 1)\n",
                "  --- stderr\n",
                "  something went wrong in the build script",
            )
        );
    }

    #[test]
    fn manifest_errors_are_reported_with_an_empty_json_stream() {
        // Cargo fails before compiling anything, so stdout stays empty.
        let messages = Messages::parse("");
        assert_eq!(
            messages.failure_diagnostics(&stderr(
                "error: unclosed table, expected `]`\n --> Cargo.toml:8:14\n"
            )),
            "error: unclosed table, expected `]`\n --> Cargo.toml:8:14"
        );
    }

    #[test]
    fn warnings_alone_do_not_stand_in_for_a_cargo_failure() {
        // rustc compiled a dependency with warnings before cargo failed on
        // its own; reporting only the warnings would bury the error.
        let messages = messages(&[diagnostic("warning", "warning: unused import\n")]);
        assert_eq!(
            messages.failure_diagnostics(&stderr(
                "   Compiling dep v0.1.0\nerror: no bin target named `nope`\n"
            )),
            "error: no bin target named `nope`"
        );
    }

    #[test]
    fn executables_are_kept_and_noise_ignored() {
        let messages = messages(&[
            json!({ "reason": "build-script-executed" }),
            json!({
                "reason": "compiler-artifact",
                "target": { "crate_types": ["bin"] },
                "executable": "/target/debug/app",
                "filenames": ["/target/debug/app"],
            }),
        ]);
        assert_eq!(
            messages.artifacts(),
            vec![PathBuf::from("/target/debug/app")]
        );
    }

    #[test]
    fn multiple_final_library_outputs_are_all_kept() {
        // Building several packages with library outputs at once uplifts each
        // of them, so the build reports the ambiguity and asks the user to
        // pass `--package`.
        let messages = messages(&[
            json!({
                "reason": "compiler-artifact",
                "target": { "crate_types": ["cdylib"] },
                "executable": null,
                "filenames": ["/target/liba.dylib"],
            }),
            json!({
                "reason": "compiler-artifact",
                "target": { "crate_types": ["cdylib"] },
                "executable": null,
                "filenames": ["/target/libb.dylib"],
            }),
        ]);
        assert_eq!(messages.artifacts().len(), 2);
    }
}
