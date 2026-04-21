mod error;

use std::{collections::BTreeSet, io::Read, path::PathBuf};

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
            let mut files = BTreeSet::new();

            for pattern in &self.files {
                for entry in glob::glob(pattern)? {
                    let entry = entry?;
                    if entry.is_dir() {
                        let dir = entry
                            .to_str()
                            .expect("directory does not have a UTF-8 compatible name");
                        for entry in glob::glob(&format!("{dir}/**/*.rs"))? {
                            files.insert(entry?);
                        }
                    } else {
                        files.insert(entry);
                    }
                }
            }

            let mut count = 0;
            for file in &files {
                if let Err(error) = format_file(file) {
                    eprintln!("{}", style(format!("{}: {error}", file.display())).red());
                    std::process::exit(1);
                } else {
                    count += 1;
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
