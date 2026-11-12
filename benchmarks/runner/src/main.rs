//! Command line runner for the cross-framework web benchmark suite.
//!
//! See `benchmarks/README.md` for usage and `benchmarks/SPEC.md` for the contract every app
//! implements.

mod config;
mod dataset;
mod doctor;
mod environment;
mod html;
mod load;
mod manifest;
mod payload;
mod report;
mod results;
mod run;
mod sampler;
mod scenario;
mod server;
mod summary;
mod units;
mod verify;

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};

use crate::{
    config::{Allocator, CliOverrides, Config, LatencyMode, Switch},
    units::Dur,
};

/// Benchmarks the Harbor Market storefront across web frameworks.
#[derive(Debug, Parser)]
#[command(name = "bench-web", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build, start and load test the selected apps, then write results and an HTML report.
    Run(Box<RunArgs>),
    /// Check that every selected app renders the same content as the reference app.
    Verify(VerifyArgs),
    /// Regenerate `report.html` from a saved results directory or `results.json` file.
    Report(ReportArgs),
    /// List the apps and scenarios.
    List(CommonArgs),
    /// Show which toolchains and apps are usable on this machine.
    Doctor(CommonArgs),
}

#[derive(Debug, Args)]
struct CommonArgs {
    /// Configuration file. Defaults to `benchmarks/bench.toml`.
    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct Selection {
    /// Only these apps (comma separated or repeated). Default: all apps.
    #[arg(long, value_delimiter = ',', value_name = "APP")]
    only: Vec<String>,
    /// Skip these apps.
    #[arg(long, value_delimiter = ',', value_name = "APP")]
    skip: Vec<String>,
    /// Do not build the apps; use the existing build output, including the allocator it was
    /// built with.
    #[arg(long)]
    skip_build: bool,
    /// Allocator for Rust apps.
    #[arg(long, value_enum)]
    allocator: Option<Allocator>,
    /// Response compression in the apps (and `Accept-Encoding` in the load generator).
    #[arg(long, value_enum)]
    compression: Option<Switch>,
    /// Worker threads or processes per app (`BENCH_THREADS`). 0 means the framework default.
    #[arg(long)]
    threads: Option<u32>,
}

#[derive(Debug, Args)]
struct RunArgs {
    #[command(flatten)]
    common: CommonArgs,
    /// Named profile from the configuration file, for example `quick`, `standard` or `full`.
    #[arg(short, long)]
    profile: Option<String>,
    #[command(flatten)]
    selection: Selection,
    /// Only these scenarios (comma separated or repeated). Default: all scenarios.
    #[arg(
        long = "scenarios",
        alias = "scenario",
        value_delimiter = ',',
        value_name = "SCENARIO"
    )]
    scenarios: Vec<String>,
    /// Skip these scenarios.
    #[arg(long, value_delimiter = ',', value_name = "SCENARIO")]
    skip_scenarios: Vec<String>,
    /// Measured duration of each run, for example `10s` or `1m`.
    #[arg(short, long)]
    duration: Option<Dur>,
    /// Warmup before the measured runs of each scenario and connection count.
    #[arg(short, long)]
    warmup: Option<Dur>,
    /// Measured runs per scenario and connection count. The report uses the median run.
    #[arg(short, long)]
    runs: Option<u32>,
    /// Concurrent connections, one throughput measurement per value (comma separated).
    #[arg(short, long, value_delimiter = ',')]
    connections: Vec<u32>,
    /// Pause after each scenario and after each app.
    #[arg(long)]
    cooldown: Option<Dur>,
    /// Enable or disable the fixed-rate latency phase.
    #[arg(long, value_enum)]
    latency: Option<Switch>,
    /// How the latency phase picks its request rate.
    #[arg(long, value_enum)]
    latency_mode: Option<LatencyMode>,
    /// Fraction of each app's measured throughput used in `relative` latency mode.
    #[arg(long)]
    latency_fraction: Option<f64>,
    /// Requests per second used in `fixed` latency mode.
    #[arg(long)]
    latency_rate: Option<u64>,
    /// Duration of the latency phase.
    #[arg(long)]
    latency_duration: Option<Dur>,
    /// Connections used by the latency phase.
    #[arg(long)]
    latency_connections: Option<u32>,
    /// Interval between memory and CPU samples of the app's process tree.
    #[arg(long)]
    sample_interval: Option<Dur>,
    /// Per-request timeout. A timed out request counts as an error.
    #[arg(long)]
    request_timeout: Option<Dur>,
    /// Skip the client payload measurement.
    #[arg(long)]
    no_payload: bool,
    /// Directory that receives one subdirectory per run.
    #[arg(short, long)]
    output_dir: Option<PathBuf>,
    /// Open the HTML report when done.
    #[arg(long)]
    open: bool,
    /// Print the resolved configuration and the plan, then exit.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct VerifyArgs {
    #[command(flatten)]
    common: CommonArgs,
    #[command(flatten)]
    selection: Selection,
    /// Print every difference instead of the first few per check.
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, Args)]
struct ReportArgs {
    /// A results directory or a `results.json` file.
    path: PathBuf,
    /// Open the report when done.
    #[arg(long)]
    open: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match dispatch(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{} {error:#}", console::style("error:").red().bold());
            ExitCode::FAILURE
        }
    }
}

fn dispatch(cli: Cli) -> Result<ExitCode> {
    let root = benchmarks_root();
    match cli.command {
        Command::Run(args) => {
            let config_path = config_path(&root, args.common.config.as_deref());
            let overrides = run_overrides(&args);
            let config = Config::load(&config_path, args.profile.as_deref(), &overrides)?;
            run::run(&root, &config, args.dry_run)
        }
        Command::Verify(args) => {
            let config_path = config_path(&root, args.common.config.as_deref());
            let overrides = selection_overrides(&args.selection);
            let config = Config::load(&config_path, None, &overrides)?;
            verify::verify(&root, &config, args.verbose)
        }
        Command::Report(args) => {
            let dir = report::regenerate(&args.path)?;
            if args.open {
                report::open(&dir.join("report.html"));
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::List(args) => {
            let config_path = config_path(&root, args.config.as_deref());
            doctor::list(&root, &config_path)?;
            Ok(ExitCode::SUCCESS)
        }
        Command::Doctor(args) => {
            let config_path = config_path(&root, args.config.as_deref());
            Ok(doctor::doctor(&root, &config_path))
        }
    }
}

/// Returns the `benchmarks/` directory. `BENCH_ROOT` overrides the location the runner was
/// built from.
fn benchmarks_root() -> PathBuf {
    if let Some(root) = std::env::var_os("BENCH_ROOT").filter(|root| !root.is_empty()) {
        return PathBuf::from(root);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn config_path(root: &Path, explicit: Option<&Path>) -> PathBuf {
    explicit.map_or_else(|| root.join("bench.toml"), Path::to_path_buf)
}

fn selection_overrides(selection: &Selection) -> CliOverrides {
    CliOverrides {
        apps: non_empty(&selection.only),
        skip: non_empty(&selection.skip),
        build: selection.skip_build.then_some(false),
        allocator: selection.allocator,
        compression: selection.compression.map(Switch::enabled),
        threads: selection.threads,
        ..CliOverrides::default()
    }
}

fn run_overrides(args: &RunArgs) -> CliOverrides {
    CliOverrides {
        scenarios: non_empty(&args.scenarios),
        skip_scenarios: non_empty(&args.skip_scenarios),
        duration: args.duration,
        warmup: args.warmup,
        runs: args.runs,
        connections: (!args.connections.is_empty()).then(|| args.connections.clone()),
        cooldown: args.cooldown,
        latency: args.latency.map(Switch::enabled),
        latency_mode: args.latency_mode,
        latency_fraction: args.latency_fraction,
        latency_rate: args.latency_rate,
        latency_duration: args.latency_duration,
        latency_connections: args.latency_connections,
        sample_interval: args.sample_interval,
        request_timeout: args.request_timeout,
        payload: args.no_payload.then_some(false),
        output_dir: args.output_dir.clone(),
        open: args.open.then_some(true),
        ..selection_overrides(&args.selection)
    }
}

fn non_empty(values: &[String]) -> Option<Vec<String>> {
    (!values.is_empty()).then(|| values.to_vec())
}

/// Reads a file, naming it in the error.
fn read_file(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))
}
