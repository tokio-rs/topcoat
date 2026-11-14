//! The `results.json` schema. Bump [`SCHEMA_VERSION`] on incompatible changes.
//!
//! Units are part of the field names: `_bytes`, `_ms`, `_secs`, `rps` (requests per second).
//! Optional values are `null` when they were not measured.

use std::{collections::BTreeMap, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::{
    config::{Config, LatencyMode},
    environment::Environment,
    manifest::Rendering,
    sampler::Usage,
    scenario::Scenario,
};

pub const SCHEMA_VERSION: u32 = 1;

/// Everything one `run` produced.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Results {
    pub schema_version: u32,
    pub runner_version: String,
    /// RFC 3339 timestamps.
    pub started_at: String,
    pub finished_at: Option<String>,
    /// `true` if the run was stopped with Ctrl-C before it finished.
    pub interrupted: bool,
    pub config: Config,
    pub environment: Environment,
    pub scenarios: Vec<ScenarioInfo>,
    pub apps: Vec<AppResult>,
}

impl Results {
    pub fn load(path: &Path) -> Result<Self> {
        let text = crate::read_file(path)?;
        serde_json::from_str(&text).with_context(|| format!("invalid {}", path.display()))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        std::fs::write(path, text).with_context(|| format!("cannot write {}", path.display()))
    }
}

/// A scenario as it was run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioInfo {
    pub id: String,
    pub title: String,
    pub description: String,
    pub method: String,
    /// `text`, `html` or `json`.
    pub response: String,
    /// Up to five of the request paths; requests rotate randomly over all of them.
    pub sample_paths: Vec<String>,
    pub path_count: usize,
    pub body_count: usize,
}

impl ScenarioInfo {
    pub fn new(scenario: &Scenario) -> Self {
        let response = serde_json::to_value(scenario.response)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_default();
        Self {
            id: scenario.id.to_owned(),
            title: scenario.title.to_owned(),
            description: scenario.description.to_owned(),
            method: scenario.method.to_owned(),
            response,
            sample_paths: scenario.paths.iter().take(5).cloned().collect(),
            path_count: scenario.paths.len(),
            body_count: scenario.bodies.len(),
        }
    }
}

/// What happened to an app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppStatus {
    /// Measured. Individual measurements can still be invalid.
    Ok,
    /// Not run, for example because a required command is missing.
    Skipped,
    BuildFailed,
    StartFailed,
    /// The app exited or stopped answering during the measurements.
    Crashed,
    Interrupted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppResult {
    pub id: String,
    pub name: String,
    pub language: String,
    pub kind: String,
    pub rendering: Rendering,
    pub description: String,
    /// Output of the manifest's `version_command`.
    pub framework_version: Option<String>,
    pub status: AppStatus,
    pub error: Option<String>,
    pub build_secs: Option<f64>,
    /// Size of the manifest's `artifacts` after the build. Manifests count different things
    /// (some include the runtime or dependencies), so this is not comparable across runtimes.
    pub artifact_bytes: Option<u64>,
    /// From spawning the process to the first `200` from `GET /health`.
    pub startup_ms: Option<f64>,
    /// Memory of the process tree after startup and `idle_settle`, before any load.
    pub idle_memory_bytes: Option<u64>,
    /// The CPUs the app's process group was pinned to (Linux with `threads` set), if any.
    #[serde(default)]
    pub cpu_affinity: Option<String>,
    pub scenarios: Vec<ScenarioResult>,
    pub payload: Vec<PagePayload>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioResult {
    pub id: String,
    /// One entry per configured connection count.
    pub throughput: Vec<Throughput>,
    pub latency: Option<LatencyPhase>,
    /// With compression on: the `Content-Encoding` the app answered with (`identity` for
    /// none). `null` when compression is off or the probe failed.
    #[serde(default)]
    pub content_encoding: Option<String>,
}

/// Closed-loop measurements at one connection count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Throughput {
    pub connections: u32,
    /// `true` if every run is valid.
    pub valid: bool,
    /// Problems seen during warmup. They do not invalidate the measurement.
    pub warmup_problems: Vec<String>,
    /// Every measured run in order.
    pub runs: Vec<Measurement>,
    /// `true` if any run used materially more cores than `threads` allowed.
    #[serde(default)]
    pub exceeded_thread_limit: bool,
    /// Index into `runs` of the run with the median `rps` (the lower one for an even count).
    pub median_index: Option<usize>,
    /// Spread of `rps` over the runs.
    pub rps_min: Option<f64>,
    pub rps_max: Option<f64>,
}

impl Throughput {
    pub fn median(&self) -> Option<&Measurement> {
        self.runs.get(self.median_index?)
    }
}

impl Measurement {
    /// Flags the measurement when the app kept more than `threads * (1 + margin) + 0.25`
    /// cores busy on average. Runtimes legitimately run a few helper threads (garbage
    /// collection, I/O), which the margin allows for.
    pub fn check_thread_limit(&mut self, threads: u32, margin: f64) {
        if threads == 0 {
            return;
        }
        let limit = f64::from(threads) * (1.0 + margin) + 0.25;
        let used = self.resources.cpu_cores_used;
        if used > limit {
            self.exceeded_thread_limit = true;
            self.problems.push(format!(
                "exceeded thread limit, not comparable: used {used:.1} cores with BENCH_THREADS={threads} (allowed up to {limit:.2})"
            ));
        }
    }
}

/// Open-loop, fixed-rate measurement with coordinated omission correction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPhase {
    pub mode: LatencyMode,
    /// The offered request rate.
    pub target_rps: u64,
    pub connections: u32,
    /// `true` if the app achieved less than 95% of `target_rps`.
    pub saturated: bool,
    pub measurement: Measurement,
}

/// One `oha` run combined with the resource usage of the app during it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Measurement {
    /// `false` if any request failed. Invalid measurements must not be compared.
    pub valid: bool,
    /// `true` if the app used materially more cores than `threads` allowed. Such a
    /// measurement is not comparable even though no request failed.
    #[serde(default)]
    pub exceeded_thread_limit: bool,
    pub problems: Vec<String>,
    pub duration_secs: f64,
    pub requests: u64,
    pub successful_requests: u64,
    /// Successful requests per second.
    pub rps: f64,
    pub latency_ms: Latency,
    /// Response bytes as received on the wire (compressed when compression is on).
    pub response_bytes: u64,
    pub bytes_per_response: f64,
    pub transfer_bytes_per_sec: f64,
    pub status_codes: BTreeMap<String, u64>,
    pub errors: BTreeMap<String, u64>,
    pub error_count: u64,
    pub resources: Resources,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Latency {
    pub mean: Option<f64>,
    pub min: Option<f64>,
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
    pub p999: Option<f64>,
    pub max: Option<f64>,
}

/// Resource usage of the app's whole process tree during a measurement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Resources {
    pub peak_memory_bytes: u64,
    pub mean_memory_bytes: u64,
    /// User plus system CPU time of the app's process tree.
    pub cpu_seconds: f64,
    /// `cpu_seconds / duration_secs`: how many cores the app kept busy on average.
    pub cpu_cores_used: f64,
    /// Successful requests per CPU-second of the app.
    pub requests_per_cpu_second: Option<f64>,
    pub samples: u32,
}

impl Resources {
    #[expect(clippy::cast_precision_loss, reason = "request counts fit in f64")]
    pub fn new(usage: &Usage, duration_secs: f64, successful: u64) -> Self {
        Self {
            peak_memory_bytes: usage.peak_memory_bytes,
            mean_memory_bytes: usage.mean_memory_bytes,
            cpu_seconds: usage.cpu_seconds,
            cpu_cores_used: if duration_secs > 0.0 {
                usage.cpu_seconds / duration_secs
            } else {
                0.0
            },
            requests_per_cpu_second: (usage.cpu_seconds > 0.0)
                .then(|| successful as f64 / usage.cpu_seconds),
            samples: usage.samples,
        }
    }
}

/// What a browser downloads for one page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagePayload {
    /// `home` or `detail`.
    pub page: String,
    pub path: String,
    pub assets: Vec<Asset>,
    /// Totals per category, the HTML document included in `html`.
    pub totals: BTreeMap<String, Sizes>,
    pub total: Sizes,
    /// Bytes of inline `<script>` and `<style>` content inside the HTML.
    pub inline_script_bytes: u64,
    pub inline_style_bytes: u64,
    /// URLs that were referenced but could not be fetched.
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub path: String,
    /// `html`, `js`, `wasm`, `css` or `other`.
    pub category: String,
    /// `document`, `html` (a tag in the page), `manifest` (`extra_assets`) or `import` (found
    /// in a script).
    pub source: String,
    pub sizes: Sizes,
}

/// Uncompressed size and the size after gzip (level 9) and brotli (quality 11).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Sizes {
    pub raw: u64,
    pub gzip: u64,
    pub brotli: u64,
}

impl std::ops::AddAssign for Sizes {
    fn add_assign(&mut self, other: Self) {
        self.raw += other.raw;
        self.gzip += other.gzip;
        self.brotli += other.brotli;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn using(cores: f64) -> Measurement {
        Measurement {
            valid: true,
            resources: Resources {
                cpu_cores_used: cores,
                ..Resources::default()
            },
            ..Measurement::default()
        }
    }

    #[test]
    fn flags_apps_that_ignore_the_thread_limit() {
        let mut ignores = using(6.0);
        ignores.check_thread_limit(1, 0.5);
        assert!(ignores.exceeded_thread_limit);
        // A request failure is a different problem: the measurement itself stays valid.
        assert!(ignores.valid);
    }

    #[test]
    fn tolerates_runtime_helper_threads() {
        // Node at one worker shows about 1.4 cores from its GC and I/O threads.
        let mut node = using(1.4);
        node.check_thread_limit(1, 0.5);
        assert!(!node.exceeded_thread_limit);

        let mut unlimited = using(16.0);
        unlimited.check_thread_limit(0, 0.5);
        assert!(!unlimited.exceeded_thread_limit);
    }
}
