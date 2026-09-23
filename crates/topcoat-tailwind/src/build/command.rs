use std::{
    path::{Path, PathBuf},
    process,
};

/// The arguments for one run of the Tailwind CLI.
///
/// Pass it to [`Executable::run`](crate::Executable::run).
/// [`BuildConfig::render`](crate::BuildConfig::render) builds one for you.
#[derive(Debug, Clone)]
pub struct Command {
    /// The input CSS file, passed with `-i`.
    pub input: PathBuf,
    /// The output CSS file, passed with `-o`.
    pub output: PathBuf,
    /// The directory the CLI scans for class names, passed with `--cwd`.
    pub cwd: PathBuf,
    /// Whether to pass `--optimize`.
    pub optimize: bool,
    /// Whether to pass `--minify`.
    pub minify: bool,
}

impl Command {
    /// Returns a process command that runs `program` with these arguments.
    pub(crate) fn to_process(&self, program: &Path) -> process::Command {
        let mut command = process::Command::new(program);
        command
            .arg("-i")
            .arg(&self.input)
            .arg("-o")
            .arg(&self.output)
            .arg("--cwd")
            .arg(&self.cwd);
        if self.optimize {
            command.arg("--optimize");
        }
        if self.minify {
            command.arg("--minify");
        }
        command
    }
}
