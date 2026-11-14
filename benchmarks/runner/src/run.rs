//! The `run` command.

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use console::style;

use crate::{
    config::{Config, LatencyMode},
    dataset::Dataset,
    environment::Environment,
    load::Load,
    manifest::{Manifest, find_on_path},
    results::{
        AppResult, AppStatus, LatencyPhase, Measurement, Resources, Results, ScenarioInfo,
        ScenarioResult, Throughput,
    },
    sampler::Sampler,
    scenario::{SCENARIO_IDS, Scenario},
    server::{self, Server},
    units::{self, Dur},
};

/// Fraction of the target rate an app must reach in the latency phase to count as keeping up.
const SATURATION_THRESHOLD: f64 = 0.95;

pub fn run(root: &Path, config: &Config, dry_run: bool) -> Result<ExitCode> {
    for id in config.scenarios.iter().chain(&config.skip_scenarios) {
        if !SCENARIO_IDS.contains(&id.as_str()) {
            bail!(
                "unknown scenario `{id}`, known scenarios: {}",
                SCENARIO_IDS.join(", ")
            );
        }
    }
    let manifests = Manifest::discover_selected(root, &config.apps)?;
    for id in config.apps.iter().chain(&config.skip) {
        if !manifests.iter().any(|manifest| &manifest.id == id) {
            let known: Vec<_> = manifests.iter().map(|m| m.id.as_str()).collect();
            bail!("unknown app `{id}`, known apps: {}", known.join(", "));
        }
    }
    let selected: Vec<&Manifest> = manifests
        .iter()
        .filter(|m| config.selects_app(&m.id))
        .collect();
    if selected.is_empty() {
        bail!(
            "no apps selected (found {} under {})",
            manifests.len(),
            root.join("apps").display()
        );
    }
    if find_on_path("oha").is_none() {
        bail!(
            "`oha` is not on PATH; enter the Nix shell (`nix develop .#benchmarks`) or install oha"
        );
    }

    let data_dir = root.join("data");
    let dataset = Dataset::load(&data_dir);
    let scenarios: Vec<Scenario> = Scenario::all(&dataset)
        .into_iter()
        .filter(|scenario| config.selects_scenario(scenario.id))
        .collect();

    print_plan(config, &selected, &scenarios);
    if dry_run {
        println!("{}", toml::to_string_pretty(config)?);
        return Ok(ExitCode::SUCCESS);
    }

    for mismatch in crate::environment::version_mismatches(&config.expected_tools) {
        warn(&format!(
            "`{}` has the wrong version: {mismatch} (see `expected_tools` in bench.toml)",
            mismatch.tool
        ));
    }
    server::install_interrupt_handler();
    server::raise_file_limit();

    let started_at = jiff::Timestamp::now();
    let name = format!(
        "{}-{}",
        started_at.strftime("%Y%m%d-%H%M%S"),
        config.profile
    );
    let dir = create_unique_dir(&config.output_root(root), &name)?;
    let logs = dir.join("logs");
    let work = dir.join("work");
    std::fs::create_dir_all(&logs).with_context(|| format!("cannot create {}", logs.display()))?;
    std::fs::create_dir_all(&work)?;
    println!("Results: {}\n", style(dir.display()).bold());

    let mut results = Results {
        schema_version: crate::results::SCHEMA_VERSION,
        runner_version: env!("CARGO_PKG_VERSION").to_owned(),
        started_at: started_at.to_string(),
        finished_at: None,
        interrupted: false,
        config: config.clone(),
        environment: Environment::collect(root, &selected, &config.expected_tools),
        scenarios: scenarios.iter().map(ScenarioInfo::new).collect(),
        apps: Vec::new(),
    };

    let bench = Bench {
        config,
        data_dir: &data_dir,
        logs: &logs,
        work: &work,
        dataset: &dataset,
        scenarios: &scenarios,
    };

    // Build everything first so build load never overlaps a measurement.
    let mut runnable = Vec::new();
    for app in &selected {
        let mut result = AppResult::new(app);
        let missing = app.missing_requirements();
        if !missing.is_empty() {
            let message = format!("missing commands: {}", missing.join(", "));
            warn(&format!("skipping {}: {message}", app.id));
            result.status = AppStatus::Skipped;
            result.error = Some(message);
            results.apps.push(result);
            continue;
        }
        if config.build && !server::interrupted() {
            println!("{} {}", style("build").cyan().bold(), app.name);
            match server::build(
                app,
                config,
                &data_dir,
                &logs.join(format!("{}.build.log", app.id)),
            ) {
                Ok(elapsed) => result.build_secs = Some(elapsed.as_secs_f64()),
                Err(error) => {
                    warn(&format!("build of {} failed: {error:#}", app.id));
                    result.status = AppStatus::BuildFailed;
                    result.error = Some(format!("{error:#}"));
                    results.apps.push(result);
                    continue;
                }
            }
        }
        result.artifact_bytes = app.artifact_bytes(config);
        result.framework_version = server::framework_version(app, config);
        runnable.push((app, results.apps.len()));
        results.apps.push(result);
    }

    for (app, index) in runnable {
        if server::interrupted() {
            results.apps[index].status = AppStatus::Interrupted;
            continue;
        }
        println!(
            "\n{} {} ({})",
            style("bench").green().bold(),
            style(&app.name).bold(),
            app.rendering
        );
        bench.bench_app(app, &mut results.apps[index]);
        results.save(&dir.join("results.json"))?;
        std::thread::sleep(config.cooldown.0);
    }

    results.interrupted = server::interrupted();
    results.finished_at = Some(jiff::Timestamp::now().to_string());
    results.save(&dir.join("results.json"))?;
    let report = dir.join("report.html");
    crate::report::write(&results, &report)?;
    let _ = std::fs::remove_dir_all(&work);
    update_latest_link(&config.output_root(root), &dir);

    crate::summary::print(&results);
    println!("\nReport:  {}", style(report.display()).bold());
    println!("Results: {}", dir.join("results.json").display());
    if config.open {
        crate::report::open(&report);
    }
    let all_valid = results.apps.iter().all(AppResult::all_valid);
    Ok(if all_valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}

struct Bench<'a> {
    config: &'a Config,
    data_dir: &'a Path,
    logs: &'a Path,
    work: &'a Path,
    dataset: &'a Dataset,
    scenarios: &'a [Scenario],
}

impl Bench<'_> {
    fn bench_app(&self, app: &Manifest, result: &mut AppResult) {
        let log = self.logs.join(format!("{}.log", app.id));
        let mut server = match Server::start(app, self.config, self.data_dir, &log) {
            Ok(server) => server,
            Err(error) => {
                warn(&format!("{} did not start: {error:#}", app.id));
                result.status = AppStatus::StartFailed;
                result.error = Some(format!("{error:#}"));
                return;
            }
        };
        let startup_ms = server.startup.as_secs_f64() * 1000.0;
        result.startup_ms = Some(startup_ms);
        result.cpu_affinity.clone_from(&server.affinity);
        std::thread::sleep(self.config.idle_settle.0);
        result.idle_memory_bytes = crate::sampler::memory_now(server.pid());
        println!(
            "  started in {}, idle memory {}",
            units::millis(startup_ms),
            result
                .idle_memory_bytes
                .map_or_else(|| "?".to_owned(), units::bytes)
        );

        for (index, scenario) in self.scenarios.iter().enumerate() {
            if server::interrupted() {
                result.status = AppStatus::Interrupted;
                break;
            }
            let scenario_result = self.bench_scenario(&mut server, scenario);
            result.scenarios.push(scenario_result);
            if server::interrupted() {
                result.status = AppStatus::Interrupted;
                break;
            }
            if !server.is_running() {
                let message = format!("the app exited during `{}`", scenario.id);
                warn(&format!(
                    "{}: {message}\n{}",
                    app.id,
                    server::tail(&log, 20)
                ));
                result.status = AppStatus::Crashed;
                result.error = Some(message);
                break;
            }
            if index + 1 < self.scenarios.len() {
                std::thread::sleep(self.config.cooldown.0);
            }
        }

        if self.config.payload && result.status == AppStatus::Ok && !server::interrupted() {
            for (page, path) in crate::payload::pages(self.dataset.in_stock) {
                match crate::payload::measure(
                    &server.base_url(),
                    page,
                    &path,
                    &app.client.extra_assets,
                    self.config.cart_count,
                ) {
                    Ok(payload) => {
                        for url in &payload.missing {
                            warn(&format!(
                                "{}: {page} references {url}, which did not load",
                                app.id
                            ));
                        }
                        println!(
                            "  {page:<8} payload {} raw, {} brotli",
                            units::bytes(payload.total.raw),
                            units::bytes(payload.total.brotli)
                        );
                        result.payload.push(payload);
                    }
                    Err(error) => warn(&format!("{}: payload of {page} failed: {error:#}", app.id)),
                }
            }
        }
        if server::interrupted() && result.status == AppStatus::Ok {
            result.status = AppStatus::Interrupted;
        }
        server.stop();
    }

    fn bench_scenario(&self, server: &mut Server, scenario: &Scenario) -> ScenarioResult {
        let config = self.config;
        let base_url = server.base_url();
        let mut result = ScenarioResult {
            id: scenario.id.to_owned(),
            throughput: Vec::new(),
            latency: None,
            content_encoding: None,
        };
        if config.compression {
            result.content_encoding =
                crate::load::probe_content_encoding(server.port, scenario, config.cart_count);
        }
        for &connections in &config.connections {
            let load = Load {
                base_url: &base_url,
                scenario,
                connections,
                duration: config.warmup,
                rate: None,
                compression: config.compression,
                cart_count: config.cart_count,
                timeout: config.request_timeout,
                work_dir: self.work,
            };
            let mut warmup_problems = Vec::new();
            if !config.warmup.0.is_zero() {
                match load.run() {
                    Ok(measurement) if !measurement.valid => warmup_problems = measurement.problems,
                    Ok(_) => {}
                    Err(error) => warmup_problems.push(format!("{error:#}")),
                }
            }
            let mut runs = Vec::new();
            for _ in 0..config.runs {
                if server::interrupted() {
                    break;
                }
                let load = Load {
                    duration: config.duration,
                    ..load
                };
                runs.push(measure(&load, server.pid(), config));
            }
            let throughput = Throughput::from_runs(connections, warmup_problems, runs);
            print_throughput(scenario.id, &throughput);
            result.throughput.push(throughput);
        }

        if config.latency.enabled && !server::interrupted() {
            result.latency = self.latency_phase(server, scenario, &result.throughput);
        }
        result
    }

    fn latency_phase(
        &self,
        server: &Server,
        scenario: &Scenario,
        throughput: &[Throughput],
    ) -> Option<LatencyPhase> {
        let latency = &self.config.latency;
        let target = match latency.mode {
            LatencyMode::Fixed => latency
                .rates
                .get(scenario.id)
                .copied()
                .unwrap_or(latency.rate),
            LatencyMode::Relative => {
                let capacity = throughput
                    .iter()
                    .filter(|t| t.valid)
                    .filter_map(|t| t.median().map(|m| m.rps))
                    .fold(0.0, f64::max);
                if capacity <= 0.0 {
                    warn(&format!(
                        "  {}: no valid throughput, skipping the latency phase",
                        scenario.id
                    ));
                    return None;
                }
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "positive rate"
                )]
                let rate = (capacity * latency.fraction).round().max(1.0) as u64;
                rate
            }
        };
        let base_url = server.base_url();
        let load = Load {
            base_url: &base_url,
            scenario,
            connections: latency.connections,
            duration: latency.duration,
            rate: Some(target),
            compression: self.config.compression,
            cart_count: self.config.cart_count,
            timeout: self.config.request_timeout,
            work_dir: self.work,
        };
        let measurement = measure(&load, server.pid(), self.config);
        #[expect(clippy::cast_precision_loss, reason = "rates fit in f64")]
        let saturated = measurement.rps < target as f64 * SATURATION_THRESHOLD;
        let phase = LatencyPhase {
            mode: latency.mode,
            target_rps: target,
            connections: latency.connections,
            saturated,
            measurement,
        };
        print_latency(scenario.id, &phase);
        Some(phase)
    }
}

/// Runs one load test while sampling the app's process tree.
fn measure(load: &Load<'_>, pid: u32, config: &Config) -> Measurement {
    let sampler = Sampler::start(pid, config.sample_interval.0);
    let outcome = load.run();
    let usage = sampler.finish();
    match outcome {
        Ok(mut measurement) => {
            measurement.resources = Resources::new(
                &usage,
                measurement.duration_secs,
                measurement.successful_requests,
            );
            measurement.check_thread_limit(config.threads, config.thread_limit_margin);
            if server::interrupted() && measurement.valid {
                measurement.valid = false;
                measurement.problems.push("interrupted".to_owned());
            }
            measurement
        }
        Err(error) => Measurement {
            valid: false,
            problems: vec![format!("{error:#}")],
            ..Measurement::default()
        },
    }
}

impl Throughput {
    fn from_runs(connections: u32, warmup_problems: Vec<String>, runs: Vec<Measurement>) -> Self {
        let valid = !runs.is_empty() && runs.iter().all(|run| run.valid);
        let mut order: Vec<usize> = (0..runs.len()).collect();
        order.sort_by(|a, b| runs[*a].rps.total_cmp(&runs[*b].rps));
        let median_index = (!order.is_empty()).then(|| order[(order.len() - 1) / 2]);
        let rps = runs.iter().map(|run| run.rps);
        Self {
            connections,
            valid,
            exceeded_thread_limit: runs.iter().any(|run| run.exceeded_thread_limit),
            warmup_problems,
            median_index,
            rps_min: rps.clone().reduce(f64::min),
            rps_max: rps.reduce(f64::max),
            runs,
        }
    }
}

impl AppResult {
    fn new(app: &Manifest) -> Self {
        Self {
            id: app.id.clone(),
            name: app.name.clone(),
            language: app.language.clone(),
            kind: app.kind.clone(),
            rendering: app.rendering,
            description: app.description.clone(),
            framework_version: None,
            status: AppStatus::Ok,
            error: None,
            build_secs: None,
            artifact_bytes: None,
            startup_ms: None,
            idle_memory_bytes: None,
            cpu_affinity: None,
            scenarios: Vec::new(),
            payload: Vec::new(),
        }
    }

    fn all_valid(&self) -> bool {
        matches!(self.status, AppStatus::Ok | AppStatus::Skipped)
            && self.scenarios.iter().all(|scenario| {
                scenario
                    .throughput
                    .iter()
                    .all(|t| t.valid && !t.exceeded_thread_limit)
                    && scenario
                        .latency
                        .as_ref()
                        .is_none_or(|l| l.measurement.valid && !l.measurement.exceeded_thread_limit)
            })
    }
}

fn print_throughput(scenario: &str, throughput: &Throughput) {
    let label = format!("{scenario} c={}", throughput.connections);
    for problem in &throughput.warmup_problems {
        warn(&format!("  {label}: warmup: {problem}"));
    }
    let Some(median) = throughput.median() else {
        return;
    };
    if throughput.valid && throughput.exceeded_thread_limit {
        let problem = throughput
            .runs
            .iter()
            .flat_map(|run| &run.problems)
            .find(|problem| problem.starts_with("exceeded thread limit"))
            .cloned()
            .unwrap_or_default();
        warn(&format!(
            "  {label}: {} req/s, {problem}",
            units::count(median.rps)
        ));
    } else if throughput.valid {
        let spread = match (throughput.rps_min, throughput.rps_max) {
            (Some(min), Some(max)) if throughput.runs.len() > 1 => {
                format!(" (runs {}..{})", units::count(min), units::count(max))
            }
            _ => String::new(),
        };
        println!(
            "  {label:<22} {:>12} req/s{spread}  p50 {:>10}  p99 {:>10}  peak {:>10}  cpu {:.1} cores",
            units::count(median.rps),
            median
                .latency_ms
                .p50
                .map_or_else(|| "-".to_owned(), units::millis),
            median
                .latency_ms
                .p99
                .map_or_else(|| "-".to_owned(), units::millis),
            units::bytes(median.resources.peak_memory_bytes),
            median.resources.cpu_cores_used,
        );
    } else {
        let problems: Vec<String> = throughput
            .runs
            .iter()
            .flat_map(|run| run.problems.iter().cloned())
            .collect();
        println!(
            "  {label:<22} {} {}",
            style("INVALID").red().bold(),
            style(problems.join("; ")).red()
        );
    }
}

fn print_latency(scenario: &str, phase: &LatencyPhase) {
    let m = &phase.measurement;
    let label = format!("{scenario} @{}/s", units::count(phase_rate(phase)));
    if m.valid {
        let note = if phase.saturated {
            style(format!(" saturated: reached {} req/s", units::count(m.rps)))
                .yellow()
                .to_string()
        } else {
            String::new()
        };
        println!(
            "  {label:<22} latency p50 {:>10}  p99 {:>10}  p99.9 {:>10}  max {:>10}{note}",
            m.latency_ms
                .p50
                .map_or_else(|| "-".to_owned(), units::millis),
            m.latency_ms
                .p99
                .map_or_else(|| "-".to_owned(), units::millis),
            m.latency_ms
                .p999
                .map_or_else(|| "-".to_owned(), units::millis),
            m.latency_ms
                .max
                .map_or_else(|| "-".to_owned(), units::millis),
        );
    } else {
        println!(
            "  {label:<22} {} {}",
            style("INVALID").red().bold(),
            style(m.problems.join("; ")).red()
        );
    }
}

#[expect(clippy::cast_precision_loss, reason = "display only")]
fn phase_rate(phase: &LatencyPhase) -> f64 {
    phase.target_rps as f64
}

fn print_plan(config: &Config, apps: &[&Manifest], scenarios: &[Scenario]) {
    let per_point = config.warmup.0 + config.duration.0 * config.runs;
    let latency = if config.latency.enabled {
        config.latency.duration.0
    } else {
        Duration::ZERO
    };
    let per_scenario = per_point * u32::try_from(config.connections.len()).unwrap_or(1)
        + latency
        + config.cooldown.0;
    let per_app = per_scenario * u32::try_from(scenarios.len()).unwrap_or(1)
        + config.idle_settle.0
        + Duration::from_secs(3);
    let total = per_app * u32::try_from(apps.len()).unwrap_or(1);
    let ids: Vec<_> = apps.iter().map(|app| app.id.as_str()).collect();
    let scenario_ids: Vec<_> = scenarios.iter().map(|s| s.id).collect();
    println!(
        "{} profile {}",
        style("plan").bold(),
        style(&config.profile).cyan()
    );
    println!("  apps       {}", ids.join(", "));
    println!("  scenarios  {}", scenario_ids.join(", "));
    println!(
        "  per point  warmup {} + {} x {} at {} connections",
        config.warmup,
        config.runs,
        config.duration,
        config
            .connections
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join("/")
    );
    if config.latency.enabled {
        let rate = match config.latency.mode {
            LatencyMode::Relative => format!(
                "{:.0}% of measured throughput",
                config.latency.fraction * 100.0
            ),
            LatencyMode::Fixed => format!("{} req/s", config.latency.rate),
        };
        println!("  latency    {} at {rate}", config.latency.duration);
    }
    println!(
        "  settings   threads {}, compression {}, allocator {} (Rust apps)",
        if config.threads == 0 {
            "default".to_owned()
        } else {
            config.threads.to_string()
        },
        if config.compression { "on" } else { "off" },
        config.allocator.name()
    );
    println!(
        "  estimate   about {} plus builds",
        Dur(Duration::from_secs(total.as_secs().div_ceil(60) * 60))
    );
}

/// Creates `<parent>/<name>`, or `<name>-2`, `<name>-3` and so on if it exists, so runs
/// started in the same second never share a directory.
fn create_unique_dir(parent: &Path, name: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(parent)
        .with_context(|| format!("cannot create {}", parent.display()))?;
    for attempt in 1u32.. {
        let dir = if attempt == 1 {
            parent.join(name)
        } else {
            parent.join(format!("{name}-{attempt}"))
        };
        match std::fs::create_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| format!("cannot create {}", dir.display()));
            }
        }
    }
    unreachable!("the attempt counter is unbounded")
}

fn update_latest_link(output_root: &Path, dir: &Path) {
    let link: PathBuf = output_root.join("latest");
    let _ = std::fs::remove_file(&link);
    if let Some(name) = dir.file_name() {
        let _ = std::os::unix::fs::symlink(name, &link);
    }
}

pub fn warn(message: &str) {
    eprintln!("{} {message}", style("warning:").yellow().bold());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_started_together_get_separate_directories() {
        let parent = std::env::temp_dir().join(format!("bench-run-dirs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&parent);
        let first = create_unique_dir(&parent, "run").unwrap();
        let second = create_unique_dir(&parent, "run").unwrap();
        assert_ne!(first, second);
        assert!(first.is_dir() && second.is_dir());
        let _ = std::fs::remove_dir_all(&parent);
    }
}
