mod error;

use std::{io::Read, path::PathBuf};

use clap::Args;

use console::style;
use topcoat_view::pretty::pretty_print_rust_str;

use crate::fmt::error::Error;

#[derive(Args)]
#[command(version, about = "Format the content of view macro invocations in Rust source files.", long_about = None)]
pub struct FmtCommand {
    #[arg(long)]
    /// If specified, reads the standard input and formats to standard output.
    stdin: bool,

    files: Vec<String>,
}

impl FmtCommand {
    pub async fn run(&self) {
        let result: Result<(), Error> = async {
            let mut count = 0;

            for pattern in &self.files {
                for entry in glob::glob(pattern)? {
                    let entry = entry?;
                    if entry.is_dir() {
                        let entry = entry
                            .to_str()
                            .expect("directory does not have a UTF-8 compatible name");
                        for entry in glob::glob(&format!("{entry}/**/*.rs"))? {
                            let entry = entry?;
                            format_file(&entry)?;
                            count += 1;
                        }
                    } else {
                        format_file(&entry)?;
                        count += 1;
                    }
                }
            }

            if self.stdin {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf = pretty_print_rust_str(&buf)?;
                print!("{buf}");
            } else {
                eprintln!(
                    "{}",
                    style(format!("successfully formatted {count} files")).green()
                )
            }
            Ok(())
        }
        .await;

        match result {
            Ok(()) => {}
            Err(error) => {
                eprintln!("{}", style(error).red())
            }
        }
    }
}

fn format_file(path: &PathBuf) -> Result<(), error::Error> {
    let input = std::fs::read_to_string(path)?;
    let output = pretty_print_rust_str(&input)?;
    std::fs::write(path, output)?;
    Ok(())
}
