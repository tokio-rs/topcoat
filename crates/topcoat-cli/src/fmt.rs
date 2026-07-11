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
        let registry = {
            // The full set of macros `topcoat fmt` knows how to format.
            const ALL_MACROS: &[&str] = &[
                "view",
                "attributes",
                "class",
                "font_face",
                "font",
                "fontsource_font_face",
                "fontsource_font",
            ];

            let mut registry = Registry::new();

            let selected: BTreeSet<&str> = match &self.macros {
                Some(names) => names.iter().map(String::as_str).collect(),
                None => ALL_MACROS.iter().copied().collect(),
            };

            for name in &selected {
                if !ALL_MACROS.contains(name) {
                    eprintln!(
                        "{}",
                        style(format!(
                            "unknown macro '{name}'; supported macros are: {}",
                            ALL_MACROS.join(", ")
                        ))
                        .red()
                    );
                    std::process::exit(1);
                }
            }

            if selected.contains("view") {
                registry.register_macro::<topcoat_view_grammar::view::View>("view");
            }
            if selected.contains("attributes") {
                registry
                    .register_macro::<topcoat_view_grammar::attributes::Attributes>("attributes");
            }
            if selected.contains("class") {
                registry.register_macro::<topcoat_view_grammar::class::Class>("class");
            }
            if selected.contains("font_face") {
                registry.register_macro::<topcoat_font_grammar::font_face::FontFace>("font_face");
            }
            if selected.contains("font") {
                registry.register_macro::<topcoat_font_grammar::font::Font>("font");
            }
            if selected.contains("fontsource_font_face") {
                registry
                    .register_macro::<topcoat_font_fontsource_grammar::font_face::FontsourceFontFace>(
                        "fontsource_font_face",
                    );
            }
            if selected.contains("fontsource_font") {
                registry.register_macro::<topcoat_font_fontsource_grammar::font::FontsourceFont>(
                    "fontsource_font",
                );
            }

            registry
        };

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
            let mut failed = 0;
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
                        failed += 1;
                        eprintln!("{}", style(format!("{}: {error}", file.display())).red());
                    }
                }
            }

            if self.stdin {
                let mut buf = String::new();
                std::io::stdin().read_to_string(&mut buf)?;
                buf = pretty_print_str(&registry, &buf)?;
                print!("{buf}");
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
