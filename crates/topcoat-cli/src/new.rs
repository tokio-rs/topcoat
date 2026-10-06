mod choice;
mod generate;
mod git;
mod input;
mod manifest;
mod name;
mod options;
mod plan;
mod publish;
#[cfg(test)]
mod temp_dir;
mod wizard;

use std::{
    io::IsTerminal,
    path::{Path, PathBuf},
};

use clap::Args;
use console::style;

use self::{
    input::{ChoiceArgs, Input},
    name::PackageName,
    options::DatabaseSetup,
    wizard::Wizard,
};

#[derive(Args)]
pub struct NewCommand {
    /// Directory to create the application in (asked for if omitted)
    path: Option<PathBuf>,
    /// Cargo package name (defaults to the directory name)
    #[arg(long)]
    name: Option<String>,
    /// Never ask questions; fail if a choice is missing
    #[arg(long)]
    no_interactive: bool,
    /// Do not initialize a Git repository
    #[arg(long)]
    no_git: bool,
    #[command(flatten)]
    choices: ChoiceArgs,
}

impl NewCommand {
    pub fn run(self) {
        if let Err(error) = self.run_inner() {
            eprintln!("{}", style(error).red());
            std::process::exit(1);
        }
    }

    fn run_inner(self) -> Result<(), String> {
        let Self {
            path,
            name,
            no_interactive,
            no_git,
            choices,
        } = self;

        // A preset answers every question, so it never prompts.
        let interactive = choices.preset().is_none()
            && !no_interactive
            && std::io::stdin().is_terminal()
            && std::io::stderr().is_terminal();
        let wizard = interactive.then(Wizard::new);

        let named = name.as_deref().map(PackageName::new).transpose()?;
        let path = match (path, &wizard) {
            (Some(path), _) => {
                check_destination(&path, named.is_some())?;
                path
            }
            (None, Some(wizard)) => wizard.destination(named.is_some())?,
            (None, None) => {
                return Err("missing destination: pass the directory to create".to_string());
            }
        };
        let package = match &named {
            Some(name) => name.clone(),
            None => PackageName::from_path(&path)?,
        };

        let mut input = Input::new(choices);
        if let Some(wizard) = &wizard {
            wizard.ask(&mut input)?;
        }
        let resolution = input.resolve()?;
        let plan = generate::generate(&package, &resolution.options)?;
        publish::publish(&plan, &path)?;

        println!(
            "{} created {} {}",
            style("+").green(),
            style(&package).bold(),
            style(format!("({})", path.display())).dim(),
        );
        for note in &resolution.notes {
            println!("{} {note}", style("!").yellow());
        }
        if !no_git && let Err(error) = git::init(&path) {
            println!(
                "{} failed to initialize a Git repository: {error}",
                style("!").yellow()
            );
        }

        let quoted = shell_quote(&path);
        let mut command = vec!["topcoat".to_string(), "new".to_string(), quoted.clone()];
        if named.is_some() {
            command.extend(["--name".to_string(), package.to_string()]);
        }
        command.extend(resolution.options.to_args());
        if no_git {
            command.push("--no-git".to_string());
        }
        println!();
        println!(
            "Equivalent command for topcoat-cli {}:",
            env!("CARGO_PKG_VERSION")
        );
        println!("  {}", style(command.join(" ")).dim());

        println!();
        let mut steps = vec![format!("cd {quoted}")];
        if let DatabaseSetup::Toasty { .. } = resolution.options.database {
            println!("Create the database tables, then start the development server:");
            steps.push("cargo run -- toasty migration generate".to_string());
            steps.push("cargo run -- toasty migration apply".to_string());
        } else {
            println!("Start the development server:");
        }
        steps.push("topcoat dev".to_string());
        for step in steps {
            println!("  {}", style(step).bold());
        }
        Ok(())
    }
}

/// Checks that `path` can be created as a new application directory. Unless `named` is
/// set, its last component must also be a valid package name.
fn check_destination(path: &Path, named: bool) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("the destination cannot be empty".to_string());
    }
    if !named {
        PackageName::from_path(path)?;
    }
    if path.exists() {
        return Err(format!("{} already exists", path.display()));
    }
    Ok(())
}

/// Quotes `path` for a POSIX shell if it contains characters other than ASCII letters,
/// digits, and `_-./`.
fn shell_quote(path: &Path) -> String {
    let path = path.to_string_lossy();
    let plain = !path.is_empty()
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_-./".contains(&byte));
    if plain {
        path.into_owned()
    } else {
        format!("'{}'", path.replace('\'', r"'\''"))
    }
}
