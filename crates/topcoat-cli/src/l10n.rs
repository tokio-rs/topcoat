mod check;

pub(crate) use check::relative_location;
use clap::{Args, Subcommand};

#[derive(Args)]
pub struct L10nCommand {
    #[command(subcommand)]
    command: L10nSubcommand,
}

#[derive(Subcommand)]
enum L10nSubcommand {
    /// Check Fluent translations used by the compiled app
    Check(check::CheckArgs),
}

impl L10nCommand {
    pub async fn run(self) {
        match self.command {
            L10nSubcommand::Check(args) => check::run(args).await,
        }
    }
}
