#![doc = include_str!("../docs/fmt.md")]

mod error;

use std::{
    collections::BTreeSet,
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    time::Instant,
};

use clap::Args;
use console::style;
use ignore::WalkBuilder;
use tokio::{io::AsyncWriteExt, process::Command};
use topcoat_core_grammar::pretty::{Registry, pretty_print_str};

use crate::{common::format, fmt::error::Error};

#[derive(Args)]
#[command(version, about = "Format the content of view macro invocations in Rust source files.", long_about = None)]
pub struct FmtCommand {
    #[arg(long)]
    /// If specified, reads the standard input and formats to standard output.
    stdin: bool,

    /// Check formatting without writing files or emitting formatted source.
    #[arg(long)]
    check: bool,

    /// Run rustfmt before formatting macro bodies. Requires rustfmt on PATH.
    #[arg(long)]
    rustfmt: bool,

    /// Comma-separated list of macro names to register and format.
    ///
    /// Defaults to all supported macros. Only macro invocations whose name
    /// appears in this list will have their bodies formatted.
    #[arg(long, value_delimiter = ',')]
    macros: Option<Vec<String>>,

    files: Vec<String>,
}

impl FmtCommand {
    pub async fn run(&self) {
        let registry = match &self.macros {
            None => format::registry(),
            Some(names) => {
                let mut registry = Registry::new();
                for name in names {
                    if !format::register(&mut registry, name) {
                        eprintln!(
                            "{}",
                            style(format!(
                                "unknown macro '{name}'; supported macros are: {}",
                                format::MACROS.join(", ")
                            ))
                            .red()
                        );
                        std::process::exit(1);
                    }
                }
                registry
            }
        };

        let start = Instant::now();
        let result: Result<(), Error> = async {
            let files = self.files()?;

            let mut count = 0;
            let mut modified = 0;
            let mut failed = 0;
            for file in &files {
                match self.format_file(file, &registry).await {
                    Ok(true) => {
                        count += 1;
                        modified += 1;
                        if self.check {
                            eprintln!("{}: needs formatting", file.display());
                        }
                    }
                    Ok(false) => {
                        count += 1;
                    }
                    Err(error) => {
                        failed += 1;
                        eprintln!("{}", style(format!("{}: {error}", file.display())).red());
                    }
                }
            }

            if self.stdin {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                let output = self.format_source(&buf, &registry, None).await?;
                if self.check {
                    count += 1;
                    if output != buf {
                        modified += 1;
                        eprintln!("stdin: needs formatting");
                    }
                } else {
                    print!("{output}");
                    return Ok(());
                }
            }

            if self.check {
                let summary = format!(
                    "checked {count} inputs ({modified} need formatting), {failed} failed in {:.0?}",
                    start.elapsed()
                );
                if modified > 0 || failed > 0 {
                    eprintln!("{}", style(summary).red());
                    std::process::exit(1);
                }
                eprintln!("{}", style(summary).green());
            } else if failed > 0 {
                eprintln!(
                    "{}",
                    style(format!(
                        "formatted {count} files ({modified} modified), {failed} failed in {:.0?}",
                        start.elapsed()
                    ))
                    .red()
                );
                std::process::exit(1);
            } else {
                eprintln!(
                    "{}",
                    style(format!(
                        "successfully formatted {count} files ({modified} modified) in {:.0?}",
                        start.elapsed()
                    ))
                    .green()
                );
            }
            Ok(())
        }
        .await;

        match result {
            Ok(()) => {}
            Err(error) => {
                eprintln!("{}", style(error).red());
                std::process::exit(1);
            }
        }
    }

    /// Collects the Rust files to format.
    fn files(&self) -> Result<BTreeSet<PathBuf>, Error> {
        let mut files = BTreeSet::new();
        if self.files.is_empty() {
            if !self.stdin {
                rust_files_in(Path::new("."), &mut files)?;
            }
            return Ok(files);
        }
        for pattern in &self.files {
            for entry in glob::glob(pattern)? {
                let entry = entry?;
                if entry.is_dir() {
                    rust_files_in(&entry, &mut files)?;
                } else {
                    files.insert(entry);
                }
            }
        }
        Ok(files)
    }

    async fn format_file(&self, path: &Path, registry: &Registry) -> Result<bool, Error> {
        let input = std::fs::read_to_string(path)?;
        let output = self.format_source(&input, registry, path.parent()).await?;
        if output == input {
            Ok(false)
        } else {
            if !self.check {
                std::fs::write(path, output)?;
            }
            Ok(true)
        }
    }

    async fn format_source(
        &self,
        input: &str,
        registry: &Registry,
        directory: Option<&Path>,
    ) -> Result<String, Error> {
        if !self.rustfmt {
            return Ok(pretty_print_str(registry, input)?);
        }

        let mut command = Command::new("rustfmt");
        command
            .args(["--emit", "stdout"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        if let Some(directory) = directory.filter(|path| !path.as_os_str().is_empty()) {
            command.current_dir(directory);
        }
        let mut child = command.spawn().map_err(Error::RustfmtIo)?;
        let mut stdin = child.stdin.take().expect("rustfmt stdin is piped");
        // Drain stdout while writing stdin so large sources cannot fill both pipes.
        let (write_result, output) = tokio::join!(
            async move { stdin.write_all(input.as_bytes()).await },
            child.wait_with_output(),
        );
        let output = output.map_err(Error::RustfmtIo)?;
        if !output.status.success() {
            return Err(Error::RustfmtFailed(output.status));
        }
        write_result.map_err(Error::RustfmtIo)?;
        let source = String::from_utf8(output.stdout).map_err(|error| {
            Error::RustfmtIo(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        Ok(pretty_print_str(registry, &source)?)
    }
}

/// Adds the Rust files under `dir` to `files`, skipping ignored files and
/// Cargo build directories.
fn rust_files_in(dir: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), Error> {
    let walk = WalkBuilder::new(dir)
        // Projects that are not in a Git repository yet still have a `.gitignore`.
        .require_git(false)
        // Cargo marks its build directories with a `CACHEDIR.TAG` file.
        .filter_entry(|entry| {
            !entry.file_type().is_some_and(|ty| ty.is_dir())
                || !entry.path().join("CACHEDIR.TAG").is_file()
        })
        .build();
    for entry in walk {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type().is_some_and(|ty| ty.is_file())
            && path.extension().is_some_and(|extension| extension == "rs")
        {
            files.insert(path.strip_prefix(".").unwrap_or(path).to_owned());
        }
    }
    Ok(())
}
