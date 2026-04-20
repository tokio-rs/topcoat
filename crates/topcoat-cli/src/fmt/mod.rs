mod error;
mod visitor;

use std::{io::Read, path::PathBuf};

use clap::Args;

use console::style;
use syn::visit::Visit;
use visitor::Visitor;

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
            let count = 0;

            // for pattern in &self.files {
            //     for entry in glob::glob(pattern)? {
            //         let entry = entry?;
            //         if entry.is_dir() {
            //             let entry = entry
            //                 .to_str()
            //                 .expect("directory does not have a UTF-8 compatible name");
            //             for entry in glob::glob(&format!("{entry}/**/*.rs"))? {
            //                 let entry = entry?;
            //                 format_file(&entry)?;
            //                 count += 1;
            //             }
            //         } else {
            //             format_file(&entry)?;
            //             count += 1;
            //         }
            //     }
            // }

            if self.stdin {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf = format_str(&buf)?;
                print!("{buf}");
            } else {
                eprintln!("{}", style("successfully formatted {} files").green())
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
    let output = format_str(&input)?;
    std::fs::write(path, output)?;
    Ok(())
}

fn format_str(input: &str) -> Result<String, Error> {
    let mut output = String::new();

    let file = syn::parse_file(input)?;
    let mut visitor = Visitor::default();
    visitor.visit_file(&file);

    if !visitor.errors.is_empty() {
        return Err(visitor.errors.into());
    }

    let mut current_index = 0;
    for replacement in visitor.replacements {
        output.push_str(&input[current_index..replacement.start]);
        output.push_str(&replacement.replacement);
        current_index = replacement.end;
    }

    output.push_str(&input[current_index..]);

    Ok(output)
}
