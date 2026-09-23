use std::{
    path::{Path, PathBuf},
    process,
};

/// One invocation of the Tailwind CLI: the arguments passed when an
/// [`Executable`](crate::build::Executable) runs it.
#[derive(Debug, Clone)]
pub struct Command {
    /// Input CSS file, passed with `-i`.
    pub input: PathBuf,
    /// Output CSS file, passed with `-o`.
    pub output: PathBuf,
    /// Working directory the CLI scans for classes, passed with `--cwd`.
    pub cwd: PathBuf,
    /// Whether to pass `--optimize`.
    pub optimize: bool,
    /// Whether to pass `--minify`.
    pub minify: bool,
}

impl Command {
    /// The `std::process` command invoking `program` with this command's
    /// arguments.
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
