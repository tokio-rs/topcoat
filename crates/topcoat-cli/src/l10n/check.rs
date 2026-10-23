use std::{
    collections::BTreeSet,
    fs::File,
    io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use clap::Args;
use console::style;
use topcoat_l10n::{Check, Location, NamespaceCheck, Severity};

use crate::common::cargo::{BuildFlags, BuildOpts};

#[derive(Args)]
pub(super) struct CheckArgs {
    #[command(flatten)]
    build: BuildFlags,
}

pub(super) async fn run(args: CheckArgs) {
    let opts = BuildOpts::from(args.build);
    let mut check = Check::binary(&build(&opts).await);

    // Cargo does not notice a new file that the path of a `messages!` matches,
    // so the build can miss it. Touching the files that invoke `messages!`
    // makes cargo expand them again.
    if !check.namespaces.is_empty() {
        let files: BTreeSet<&Path> = check
            .namespaces
            .iter()
            .map(|namespace| namespace.location.file.as_path())
            .collect();
        for file in files {
            if let Err(error) = touch(file) {
                eprintln!(
                    "{} could not touch {}: {error}",
                    style("warning:").yellow().bold(),
                    relative(file).display()
                );
            }
        }
        check = Check::binary(&build(&opts).await);
    }

    print(&check);
    if check.has_errors() {
        std::process::exit(1);
    }
}

/// Builds the app and reads the binary. Exits if the build fails.
async fn build(opts: &BuildOpts) -> Vec<u8> {
    let (_, bytes) = opts
        .build_and_read(|_, _| {})
        .await
        .unwrap_or_else(|e| e.print_and_exit());
    bytes
}

/// Updates a file's modification time to now, leaving its contents unchanged.
fn touch(path: &Path) -> io::Result<()> {
    File::options()
        .write(true)
        .open(path)?
        .set_modified(SystemTime::now())
}

/// Prints translation coverage and problems for each checked namespace.
fn print(check: &Check) {
    if check.namespaces.is_empty() {
        println!("the binary has no `messages!`");
        return;
    }
    let (mut errors, mut warnings) = (0, 0);
    for namespace in &check.namespaces {
        print_namespace(namespace);
        for problem in &namespace.problems {
            match problem.severity {
                Severity::Error => errors += 1,
                Severity::Warning => warnings += 1,
            }
        }
    }
    println!(
        "{} {}, {} {}",
        errors,
        if errors == 1 { "error" } else { "errors" },
        warnings,
        if warnings == 1 { "warning" } else { "warnings" },
    );
}

fn print_namespace(namespace: &NamespaceCheck) {
    println!(
        "{} {}",
        style(format!("messages!({:?})", namespace.pattern)).bold(),
        style(format!("at {}", relative_location(&namespace.location))).dim(),
    );
    let width = namespace
        .coverage
        .iter()
        .map(|coverage| coverage.locale.to_string().len())
        .max()
        .unwrap_or(0);
    for coverage in &namespace.coverage {
        let found = coverage.keys - coverage.missing.len();
        let count = format!("{found}/{}", coverage.keys);
        let count = if coverage.missing.is_empty() {
            style(count).green()
        } else {
            style(count).red()
        };
        print!("  {:width$}  {count}", coverage.locale.to_string());
        if !coverage.missing.is_empty() {
            print!("  missing: {}", coverage.missing.join(", "));
        }
        println!();
    }
    println!();
    for problem in &namespace.problems {
        let severity = match problem.severity {
            Severity::Error => style("error").red().bold(),
            Severity::Warning => style("warning").yellow().bold(),
        };
        println!("{severity}: {}", problem.message);
        println!(
            "  {} {}",
            style("-->").blue(),
            relative_location(&problem.location)
        );
    }
    if !namespace.problems.is_empty() {
        println!();
    }
}

/// Makes the file path in `location` relative to the current directory when possible.
pub(crate) fn relative_location(location: &Location) -> Location {
    Location {
        file: relative(&location.file),
        ..location.clone()
    }
}

/// Makes `path` relative to the current directory if it is inside that directory.
fn relative(path: &Path) -> PathBuf {
    std::env::current_dir()
        .ok()
        .and_then(|dir| path.strip_prefix(dir).ok())
        .unwrap_or(path)
        .to_path_buf()
}
