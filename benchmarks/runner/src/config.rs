//! Configuration: built-in defaults, `bench.toml`, a named profile and command line flags,
//! applied in that order.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::units::Dur;

/// Global allocator compiled into Rust apps through cargo features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Allocator {
    System,
    Jemalloc,
    Mimalloc,
}

impl Allocator {
    /// The cargo arguments substituted for `{features}` in Rust build commands.
    pub fn cargo_features(self) -> &'static str {
        match self {
            Self::System => "",
            Self::Jemalloc => "--features jemalloc",
            Self::Mimalloc => "--features mimalloc",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Jemalloc => "jemalloc",
            Self::Mimalloc => "mimalloc",
        }
    }
}

/// How the latency phase picks its request rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LatencyMode {
    /// A fraction of the throughput the app reached in the same run.
    Relative,
    /// The same absolute rate for every app.
    Fixed,
}

/// An `on`/`off` command line switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Switch {
    On,
    Off,
}

impl Switch {
    pub fn enabled(self) -> bool {
        self == Self::On
    }
}

/// The fully resolved configuration of one invocation. It is stored in `results.json`.
#[expect(
    clippy::struct_excessive_bools,
    reason = "mirrors the configuration file"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub profile: String,
    pub apps: Vec<String>,
    pub skip: Vec<String>,
    pub scenarios: Vec<String>,
    pub skip_scenarios: Vec<String>,
    pub warmup: Dur,
    pub duration: Dur,
    pub runs: u32,
    pub connections: Vec<u32>,
    pub threads: u32,
    pub compression: bool,
    pub allocator: Allocator,
    pub cooldown: Dur,
    pub build: bool,
    pub payload: bool,
    pub output_dir: PathBuf,
    pub open: bool,
    pub sample_interval: Dur,
    pub request_timeout: Dur,
    pub idle_settle: Dur,
    pub cart_count: u32,
    /// A measurement is flagged when the app used more than `threads * (1 + margin) + 0.25`
    /// cores.
    #[serde(default = "default_thread_limit_margin")]
    pub thread_limit_margin: f64,
    /// Tool versions the apps are expected to be built with, as `command -> version`.
    #[serde(default)]
    pub expected_tools: BTreeMap<String, String>,
    pub latency: LatencyConfig,
}

/// Settings of the open-loop, fixed-rate latency phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    pub enabled: bool,
    pub mode: LatencyMode,
    pub fraction: f64,
    pub rate: u64,
    pub rates: BTreeMap<String, u64>,
    pub duration: Dur,
    pub connections: u32,
}

impl Config {
    /// Loads `path`, applies `profile` (or the file's default profile) and then `cli`.
    pub fn load(path: &Path, profile: Option<&str>, cli: &CliOverrides) -> Result<Self> {
        let file = if path.exists() {
            let text = crate::read_file(path)?;
            ConfigFile::parse(&text).with_context(|| format!("invalid {}", path.display()))?
        } else {
            eprintln!(
                "{} {} not found, using built-in defaults",
                console::style("warning:").yellow().bold(),
                path.display()
            );
            ConfigFile::default()
        };

        let profile_name = profile
            .map(str::to_owned)
            .or(file.profile.clone())
            .unwrap_or_else(|| "standard".to_owned());
        let profile_layer = match file.profiles.get(&profile_name) {
            Some(layer) => layer.clone(),
            None if profile.is_none() => Layer::default(),
            None => {
                let known: Vec<_> = file.profiles.keys().map(String::as_str).collect();
                bail!(
                    "unknown profile `{profile_name}`, known profiles: {}",
                    known.join(", ")
                );
            }
        };

        let mut layer = Layer::builtin();
        layer.apply(file.defaults);
        layer.apply(profile_layer);
        layer.apply(cli.to_layer());

        let latency = layer.latency.unwrap_or_default();
        let config = Self {
            profile: profile_name,
            apps: layer.apps.unwrap_or_default(),
            skip: layer.skip.unwrap_or_default(),
            scenarios: layer.scenarios.unwrap_or_default(),
            skip_scenarios: layer.skip_scenarios.unwrap_or_default(),
            warmup: required(layer.warmup, "warmup")?,
            duration: required(layer.duration, "duration")?,
            runs: required(layer.runs, "runs")?,
            connections: required(layer.connections, "connections")?,
            threads: required(layer.threads, "threads")?,
            compression: required(layer.compression, "compression")?,
            allocator: required(layer.allocator, "allocator")?,
            cooldown: required(layer.cooldown, "cooldown")?,
            build: required(layer.build, "build")?,
            payload: required(layer.payload, "payload")?,
            output_dir: required(layer.output_dir, "output_dir")?,
            open: required(layer.open, "open")?,
            sample_interval: required(layer.sample_interval, "sample_interval")?,
            request_timeout: required(layer.request_timeout, "request_timeout")?,
            idle_settle: required(layer.idle_settle, "idle_settle")?,
            cart_count: required(layer.cart_count, "cart_count")?,
            thread_limit_margin: required(layer.thread_limit_margin, "thread_limit_margin")?,
            expected_tools: file.expected_tools.clone(),
            latency: LatencyConfig {
                enabled: required(latency.enabled, "latency.enabled")?,
                mode: required(latency.mode, "latency.mode")?,
                fraction: required(latency.fraction, "latency.fraction")?,
                rate: required(latency.rate, "latency.rate")?,
                rates: latency.rates.unwrap_or_default(),
                duration: required(latency.duration, "latency.duration")?,
                connections: required(latency.connections, "latency.connections")?,
            },
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.runs == 0 {
            bail!("`runs` must be at least 1");
        }
        if self.connections.is_empty() || self.connections.contains(&0) {
            bail!("`connections` must list at least one positive value");
        }
        if self.duration.0.is_zero() {
            bail!("`duration` must be positive");
        }
        if !(self.latency.fraction > 0.0 && self.latency.fraction <= 1.0) {
            bail!("`latency.fraction` must be in (0, 1]");
        }
        if self.latency.connections == 0 || self.latency.rate == 0 {
            bail!("`latency.connections` and `latency.rate` must be positive");
        }
        if self.thread_limit_margin.is_nan() || self.thread_limit_margin < 0.0 {
            bail!("`thread_limit_margin` must not be negative");
        }
        if self.sample_interval.0.as_millis() < 10 {
            bail!("`sample_interval` must be at least 10ms");
        }
        Ok(())
    }

    /// The directory that receives result directories, resolved against `benchmarks/`.
    pub fn output_root(&self, root: &Path) -> PathBuf {
        if self.output_dir.is_absolute() {
            self.output_dir.clone()
        } else {
            root.join(&self.output_dir)
        }
    }

    /// Returns `true` if the app with this id is selected.
    pub fn selects_app(&self, id: &str) -> bool {
        (self.apps.is_empty() || self.apps.iter().any(|app| app == id))
            && !self.skip.iter().any(|app| app == id)
    }

    /// Returns `true` if the scenario with this id is selected.
    pub fn selects_scenario(&self, id: &str) -> bool {
        (self.scenarios.is_empty() || self.scenarios.iter().any(|s| s == id))
            && !self.skip_scenarios.iter().any(|s| s == id)
    }
}

fn default_thread_limit_margin() -> f64 {
    0.5
}

fn required<T>(value: Option<T>, name: &str) -> Result<T> {
    value.with_context(|| format!("missing configuration value `{name}`"))
}

/// The parsed `bench.toml`: defaults at the top level plus `[profiles.<name>]` tables.
#[derive(Debug, Default)]
struct ConfigFile {
    profile: Option<String>,
    defaults: Layer,
    profiles: BTreeMap<String, Layer>,
    expected_tools: BTreeMap<String, String>,
}

impl ConfigFile {
    fn parse(text: &str) -> Result<Self> {
        let mut table: toml::Table = toml::from_str(text)?;
        let profile = match table.remove("profile") {
            Some(toml::Value::String(name)) => Some(name),
            Some(_) => bail!("`profile` must be a string"),
            None => None,
        };
        let profiles = match table.remove("profiles") {
            Some(value) => BTreeMap::<String, Layer>::deserialize(value)
                .context("invalid `[profiles]` table")?,
            None => BTreeMap::new(),
        };
        let expected_tools = match table.remove("expected_tools") {
            Some(value) => BTreeMap::<String, String>::deserialize(value)
                .context("invalid `[expected_tools]` table")?,
            None => BTreeMap::new(),
        };
        let defaults = Layer::deserialize(toml::Value::Table(table))?;
        Ok(Self {
            profile,
            defaults,
            profiles,
            expected_tools,
        })
    }
}

/// A partial configuration. Later layers override the fields they set.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Layer {
    apps: Option<Vec<String>>,
    skip: Option<Vec<String>>,
    scenarios: Option<Vec<String>>,
    skip_scenarios: Option<Vec<String>>,
    warmup: Option<Dur>,
    duration: Option<Dur>,
    runs: Option<u32>,
    connections: Option<Vec<u32>>,
    threads: Option<u32>,
    compression: Option<bool>,
    allocator: Option<Allocator>,
    cooldown: Option<Dur>,
    build: Option<bool>,
    payload: Option<bool>,
    output_dir: Option<PathBuf>,
    open: Option<bool>,
    sample_interval: Option<Dur>,
    request_timeout: Option<Dur>,
    idle_settle: Option<Dur>,
    cart_count: Option<u32>,
    thread_limit_margin: Option<f64>,
    latency: Option<LatencyLayer>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct LatencyLayer {
    enabled: Option<bool>,
    mode: Option<LatencyMode>,
    fraction: Option<f64>,
    rate: Option<u64>,
    rates: Option<BTreeMap<String, u64>>,
    duration: Option<Dur>,
    connections: Option<u32>,
}

macro_rules! merge {
    ($target:expr, $source:expr, [$($field:ident),* $(,)?]) => {
        $(if $source.$field.is_some() { $target.$field = $source.$field; })*
    };
}

impl Layer {
    /// Defaults used when `bench.toml` does not set a value. They match the documented
    /// defaults in `bench.toml`.
    fn builtin() -> Self {
        let dur = |text: &str| Some(text.parse::<Dur>().expect("valid built-in duration"));
        Self {
            apps: Some(Vec::new()),
            skip: Some(Vec::new()),
            scenarios: Some(Vec::new()),
            skip_scenarios: Some(Vec::new()),
            warmup: dur("5s"),
            duration: dur("10s"),
            runs: Some(1),
            connections: Some(vec![64]),
            threads: Some(0),
            compression: Some(false),
            allocator: Some(Allocator::Jemalloc),
            cooldown: dur("2s"),
            build: Some(true),
            payload: Some(true),
            output_dir: Some(PathBuf::from("results")),
            open: Some(false),
            sample_interval: dur("250ms"),
            request_timeout: dur("10s"),
            idle_settle: dur("2s"),
            cart_count: Some(3),
            thread_limit_margin: Some(0.5),
            latency: Some(LatencyLayer {
                enabled: Some(false),
                mode: Some(LatencyMode::Relative),
                fraction: Some(0.5),
                rate: Some(1000),
                rates: Some(BTreeMap::new()),
                duration: dur("10s"),
                connections: Some(64),
            }),
        }
    }

    fn apply(&mut self, other: Self) {
        merge!(
            self,
            other,
            [
                apps,
                skip,
                scenarios,
                skip_scenarios,
                warmup,
                duration,
                runs,
                connections,
                threads,
                compression,
                allocator,
                cooldown,
                build,
                payload,
                output_dir,
                open,
                sample_interval,
                request_timeout,
                idle_settle,
                cart_count,
                thread_limit_margin,
            ]
        );
        if let Some(other_latency) = other.latency {
            let latency = self.latency.get_or_insert_with(LatencyLayer::default);
            merge!(
                latency,
                other_latency,
                [enabled, mode, fraction, rate, rates, duration, connections,]
            );
        }
    }
}

/// Values set on the command line.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub apps: Option<Vec<String>>,
    pub skip: Option<Vec<String>>,
    pub scenarios: Option<Vec<String>>,
    pub skip_scenarios: Option<Vec<String>>,
    pub warmup: Option<Dur>,
    pub duration: Option<Dur>,
    pub runs: Option<u32>,
    pub connections: Option<Vec<u32>>,
    pub threads: Option<u32>,
    pub compression: Option<bool>,
    pub allocator: Option<Allocator>,
    pub cooldown: Option<Dur>,
    pub build: Option<bool>,
    pub payload: Option<bool>,
    pub output_dir: Option<PathBuf>,
    pub open: Option<bool>,
    pub sample_interval: Option<Dur>,
    pub request_timeout: Option<Dur>,
    pub latency: Option<bool>,
    pub latency_mode: Option<LatencyMode>,
    pub latency_fraction: Option<f64>,
    pub latency_rate: Option<u64>,
    pub latency_duration: Option<Dur>,
    pub latency_connections: Option<u32>,
}

impl CliOverrides {
    fn to_layer(&self) -> Layer {
        Layer {
            apps: self.apps.clone(),
            skip: self.skip.clone(),
            scenarios: self.scenarios.clone(),
            skip_scenarios: self.skip_scenarios.clone(),
            warmup: self.warmup,
            duration: self.duration,
            runs: self.runs,
            connections: self.connections.clone(),
            threads: self.threads,
            compression: self.compression,
            allocator: self.allocator,
            cooldown: self.cooldown,
            build: self.build,
            payload: self.payload,
            output_dir: self.output_dir.clone(),
            open: self.open,
            sample_interval: self.sample_interval,
            request_timeout: self.request_timeout,
            idle_settle: None,
            cart_count: None,
            thread_limit_margin: None,
            latency: Some(LatencyLayer {
                enabled: self.latency,
                mode: self.latency_mode,
                fraction: self.latency_fraction,
                rate: self.latency_rate,
                rates: None,
                duration: self.latency_duration,
                connections: self.latency_connections,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(text: &str, profile: Option<&str>, cli: &CliOverrides) -> Result<Config> {
        // Tests run in parallel, so every call writes its own file.
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("bench-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let index = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = dir.join(format!("{index}.toml"));
        std::fs::write(&path, text).unwrap();
        Config::load(&path, profile, cli)
    }

    const FILE: &str = r#"
        profile = "standard"
        duration = "10s"
        connections = [64]
        [latency]
        rate = 500
        [profiles.standard]
        [profiles.quick]
        duration = "2s"
        [profiles.quick.latency]
        enabled = true
    "#;

    #[test]
    fn later_layers_override_earlier_ones() {
        let quick = load(FILE, Some("quick"), &CliOverrides::default()).unwrap();
        assert_eq!(quick.duration, "2s".parse().unwrap());
        assert!(quick.latency.enabled);
        // Values the profile does not set fall through to the file and the built-ins.
        assert_eq!(quick.latency.rate, 500);
        assert_eq!(quick.connections, vec![64]);

        let cli = CliOverrides {
            duration: Some("7s".parse().unwrap()),
            latency: Some(false),
            ..CliOverrides::default()
        };
        let overridden = load(FILE, Some("quick"), &cli).unwrap();
        assert_eq!(overridden.duration, "7s".parse().unwrap());
        assert!(!overridden.latency.enabled);
    }

    #[test]
    fn rejects_unknown_profiles_and_keys() {
        assert!(load(FILE, Some("nope"), &CliOverrides::default()).is_err());
        let typo = "[profiles.quick]\ndurration = \"1s\"\n";
        assert!(load(typo, Some("quick"), &CliOverrides::default()).is_err());
    }

    #[test]
    fn selection_honors_only_and_skip() {
        let cli = CliOverrides {
            apps: Some(vec!["a".into(), "b".into()]),
            skip: Some(vec!["b".into()]),
            ..CliOverrides::default()
        };
        let config = load(FILE, None, &cli).unwrap();
        assert!(config.selects_app("a"));
        assert!(!config.selects_app("b"));
        assert!(!config.selects_app("c"));
    }
}
