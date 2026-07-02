#![doc = include_str!("../docs/fmt.md")]

mod error;

use std::{collections::BTreeSet, io::Read, path::PathBuf, time::Instant};
use topcoat_pretty::{Registry, pretty_print_str};

use clap::Args;

use console::style;

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
        let mut registry = Registry::new();
        registry.register_macro::<topcoat_view::ast::view::View>("view");
        registry.register_macro::<topcoat_view::ast::attributes::Attributes>("attributes");
        registry.register_macro::<topcoat_font::ast::font_face::FontFace>("font_face");
        registry.register_macro::<topcoat_font::ast::font::Font>("font");
        registry.register_macro::<topcoat_font_fontsource::ast::font_face::FontsourceFontFace>(
            "fontsource_font_face",
        );
        registry.register_macro::<topcoat_font_fontsource::ast::font::FontsourceFont>(
            "fontsource_font",
        );

        let start = Instant::now();
        let result: Result<(), Error> = async {
            let mut files = BTreeSet::new();

            let patterns: Vec<&str> = if self.files.is_empty() && !self.stdin {
                vec!["**/*.rs"]
            } else {
                self.files.iter().map(String::as_str).collect()
            };

            for pattern in patterns {
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
            let mut modified = 0;
            for file in &files {
                match format_file(file, &registry) {
                    Ok(true) => {
                        count += 1;
                        modified += 1;
                    }
                    Ok(false) => {
                        count += 1;
                    }
                    Err(error) => {
                        eprintln!("{}", style(format!("{}: {error}", file.display())).red());
                        std::process::exit(1);
                    }
                }
            }

            if self.stdin {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf = pretty_print_str(&registry, &buf)?;
                print!("{buf}");
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
            }
        }
    }
}

fn format_file(path: &PathBuf, registry: &Registry) -> Result<bool, error::Error> {
    let input = std::fs::read_to_string(path)?;
    let output = pretty_print_str(registry, &input)?;
    if output == input {
        Ok(false)
    } else {
        std::fs::write(path, output)?;
        Ok(true)
    }
}
