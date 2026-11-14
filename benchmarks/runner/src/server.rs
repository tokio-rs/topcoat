//! Building, starting and stopping apps.
//!
//! Each app runs in its own process group so that the whole tree (workers forked by Node,
//! Ruby or Python included) can be signalled at once. Groups are registered globally so the
//! Ctrl-C handler and panics never leave servers behind.

use std::{
    collections::BTreeMap,
    fs::File,
    io::{BufRead, BufReader, Seek, SeekFrom},
    net::TcpListener,
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use anyhow::{Context, Result, bail};
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};

use crate::{
    config::Config,
    manifest::{Manifest, find_on_path},
};

/// Process groups of apps that are currently running.
static GROUPS: Mutex<Vec<i32>> = Mutex::new(Vec::new());

/// Set once the user pressed Ctrl-C.
static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Installs a Ctrl-C handler. The first press stops running apps and lets the runner write
/// partial results; the second press exits immediately.
pub fn install_interrupt_handler() {
    let result = ctrlc::set_handler(|| {
        let again = INTERRUPTED.swap(true, Ordering::SeqCst);
        kill_all_groups();
        if again {
            std::process::exit(130);
        }
        eprintln!(
            "\n{} interrupted, stopping apps and writing partial results (press Ctrl-C again to quit now)",
            console::style("warning:").yellow().bold()
        );
    });
    if let Err(error) = result {
        eprintln!("warning: cannot install the Ctrl-C handler: {error}");
    }
}

pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

fn kill_all_groups() {
    let groups = GROUPS
        .lock()
        .map_or_else(|_| Vec::new(), |groups| groups.clone());
    for group in groups {
        let _ = killpg(Pid::from_raw(group), Signal::SIGKILL);
    }
}

/// Raises the open file limit so the load generator can hold many connections.
pub fn raise_file_limit() {
    use nix::sys::resource::{Resource, getrlimit, setrlimit};

    if let Ok((soft, hard)) = getrlimit(Resource::RLIMIT_NOFILE) {
        let target = hard.min(65_536);
        if soft < target {
            let _ = setrlimit(Resource::RLIMIT_NOFILE, target, hard);
        }
    }
}

/// The environment from `SPEC.md` passed to every app.
pub fn bench_env(config: &Config, data_dir: &Path, port: Option<u16>) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    if let Some(port) = port {
        env.insert("PORT".to_owned(), port.to_string());
    }
    env.insert("BENCH_DATA_DIR".to_owned(), data_dir.display().to_string());
    env.insert("BENCH_THREADS".to_owned(), config.threads.to_string());
    let compression = if config.compression { "on" } else { "off" };
    env.insert("BENCH_COMPRESSION".to_owned(), compression.to_owned());
    env
}

/// Returns `true` for variables that cargo sets for the process it runs (`cargo run`) or for
/// build scripts. Apps must not inherit them from the runner: some frameworks change behavior
/// when they think they run under cargo. User configuration such as `CARGO_HOME`,
/// `CARGO_TARGET_DIR`, `CARGO_BUILD_*`, `CARGO_NET_*` and `RUSTUP_*` is not affected.
pub fn is_cargo_injected(name: &str) -> bool {
    const EXACT: [&str; 13] = [
        "CARGO",
        "CARGO_MANIFEST_DIR",
        "CARGO_MANIFEST_PATH",
        "CARGO_MANIFEST_LINKS",
        "CARGO_CRATE_NAME",
        "CARGO_BIN_NAME",
        "CARGO_PRIMARY_PACKAGE",
        "CARGO_TARGET_TMPDIR",
        "CARGO_RUSTC_CURRENT_DIR",
        "CARGO_MAKEFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "CARGO_SBOM_PATH",
        "OUT_DIR",
    ];
    const PREFIXES: [&str; 5] = [
        "CARGO_PKG_",
        "CARGO_BIN_EXE_",
        "CARGO_CFG_",
        "CARGO_FEATURE_",
        "DEP_",
    ];
    EXACT.contains(&name) || PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// A `sh -c <command>` in the app directory, without cargo-injected variables, with the
/// manifest's `env` and then the `SPEC.md` environment.
fn shell(
    app: &Manifest,
    config: &Config,
    command: &str,
    env: &BTreeMap<String, String>,
    bench: BTreeMap<String, String>,
    affinity: Option<&str>,
) -> Command {
    let mut process = match affinity {
        Some(cpus) => {
            let mut process = Command::new("taskset");
            process.args(["-c", cpus, "sh"]);
            process
        }
        None => Command::new("sh"),
    };
    process.arg("-c").arg(command).current_dir(&app.dir);
    for (name, _) in std::env::vars_os() {
        if name.to_str().is_some_and(is_cargo_injected) {
            process.env_remove(name);
        }
    }
    for (name, value) in env {
        process.env(name, app.expand(value, config));
    }
    process.envs(bench);
    process
}

/// The CPU list the app is pinned to when `threads` limits it, if the platform allows pinning
/// (Linux with `taskset`).
pub fn cpu_affinity(config: &Config) -> Option<String> {
    if config.threads == 0 || !cfg!(target_os = "linux") || find_on_path("taskset").is_none() {
        return None;
    }
    let cores = std::thread::available_parallelism().map_or(1, usize::from);
    let count = usize::try_from(config.threads)
        .unwrap_or(usize::MAX)
        .min(cores);
    Some(if count == 1 {
        "0".to_owned()
    } else {
        format!("0-{}", count - 1)
    })
}

/// Runs the app's build commands, appending their output to `log`. Returns the build time.
pub fn build(app: &Manifest, config: &Config, data_dir: &Path, log: &Path) -> Result<Duration> {
    let started = Instant::now();
    for command in &app.build.commands {
        let command = app.expand(command, config);
        eprintln!("    {} {command}", console::style("$").dim());
        let output = log_file(log)?;
        let status = shell(
            app,
            config,
            &command,
            &app.build.env,
            bench_env(config, data_dir, None),
            None,
        )
        .stdin(Stdio::null())
        .stdout(output.try_clone()?)
        .stderr(output)
        .status()
        .with_context(|| format!("cannot run `{command}`"))?;
        if interrupted() {
            bail!("interrupted");
        }
        if !status.success() {
            bail!("`{command}` failed with {status}\n{}", tail(log, 30));
        }
    }
    Ok(started.elapsed())
}

/// Runs the manifest's `version_command`, if any, and returns its trimmed output.
pub fn framework_version(app: &Manifest, config: &Config) -> Option<String> {
    let command = app.expand(app.version_command.as_ref()?, config);
    let output = shell(app, config, &command, &app.build.env, BTreeMap::new(), None)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (output.status.success() && !text.is_empty()).then(|| text.chars().take(200).collect())
}

/// A running app.
pub struct Server {
    child: Child,
    group: i32,
    pub port: u16,
    pub startup: Duration,
    /// The CPUs the app is pinned to, if any.
    pub affinity: Option<String>,
    stop_timeout: Duration,
    log: PathBuf,
}

impl Server {
    /// Starts the app on a free port and waits until `GET /health` answers `200`.
    pub fn start(app: &Manifest, config: &Config, data_dir: &Path, log: &Path) -> Result<Self> {
        let port = free_port()?;
        let command = app.expand(&app.run.command, config);
        let output = log_file(log)?;
        let affinity = cpu_affinity(config);
        let started = Instant::now();
        let bench = bench_env(config, data_dir, Some(port));
        let exec = format!("exec {command}");
        let child = shell(app, config, &exec, &app.run.env, bench, affinity.as_deref())
            .stdin(Stdio::null())
            .stdout(output.try_clone()?)
            .stderr(output)
            .process_group(0)
            .spawn()
            .with_context(|| format!("cannot start `{command}`"))?;
        let group = i32::try_from(child.id()).context("pid out of range")?;
        if let Ok(mut groups) = GROUPS.lock() {
            groups.push(group);
        }
        let mut server = Self {
            child,
            group,
            port,
            startup: Duration::ZERO,
            affinity,
            stop_timeout: Duration::from_secs(app.run.stop_timeout_secs),
            log: log.to_path_buf(),
        };
        server.startup = server
            .wait_ready(started, Duration::from_secs(app.run.ready_timeout_secs))
            .inspect_err(|_| server.kill())?;
        Ok(server)
    }

    fn wait_ready(&mut self, started: Instant, timeout: Duration) -> Result<Duration> {
        let agent = probe_agent();
        let url = format!("{}/health", self.base_url());
        loop {
            if let Ok(response) = agent.get(&url).call()
                && response.status() == 200
            {
                return Ok(started.elapsed());
            }
            if let Some(status) = self.child.try_wait()? {
                bail!(
                    "the app exited during startup with {status}\n{}",
                    tail(&self.log, 30)
                );
            }
            if interrupted() {
                bail!("interrupted");
            }
            if started.elapsed() > timeout {
                bail!(
                    "the app did not answer GET /health with 200 within {}s\n{}",
                    timeout.as_secs(),
                    tail(&self.log, 30)
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// The pid of the process at the root of the app's process tree.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Returns `true` while the root process is alive.
    pub fn is_running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    /// Sends `SIGTERM` to the process group, waits for the root to exit and then kills
    /// whatever is left.
    pub fn stop(mut self) {
        let _ = killpg(Pid::from_raw(self.group), Signal::SIGTERM);
        let deadline = Instant::now() + self.stop_timeout;
        while Instant::now() < deadline {
            if !self.is_running() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        self.kill();
    }

    fn kill(&mut self) {
        let _ = killpg(Pid::from_raw(self.group), Signal::SIGKILL);
        let _ = self.child.wait();
        if let Ok(mut groups) = GROUPS.lock() {
            groups.retain(|group| *group != self.group);
        }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.kill();
    }
}

/// An HTTP client for readiness probes with short timeouts.
fn probe_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_millis(500)))
        .build()
        .new_agent()
}

fn free_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0").context("cannot find a free port")?;
    Ok(listener.local_addr()?.port())
}

fn log_file(path: &Path) -> Result<File> {
    File::options()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("cannot open {}", path.display()))
}

/// The last `lines` lines of a log file, indented for display.
pub fn tail(path: &Path, lines: usize) -> String {
    let Ok(mut file) = File::open(path) else {
        return String::new();
    };
    let len = file.metadata().map_or(0, |meta| meta.len());
    let _ = file.seek(SeekFrom::Start(len.saturating_sub(64 * 1024)));
    let all: Vec<String> = BufReader::new(file).lines().map_while(Result::ok).collect();
    let start = all.len().saturating_sub(lines);
    let mut out = format!("    --- last lines of {} ---", path.display());
    for line in &all[start..] {
        out.push_str("\n    ");
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_cargo_run_variables_but_keeps_user_configuration() {
        for injected in [
            "CARGO",
            "CARGO_MANIFEST_DIR",
            "CARGO_PKG_NAME",
            "CARGO_BIN_EXE_bench-web",
            "CARGO_CRATE_NAME",
            "CARGO_PRIMARY_PACKAGE",
            "OUT_DIR",
        ] {
            assert!(is_cargo_injected(injected), "{injected} should be stripped");
        }
        for kept in [
            "CARGO_HOME",
            "CARGO_TARGET_DIR",
            "CARGO_BUILD_JOBS",
            "CARGO_NET_OFFLINE",
            "RUSTUP_TOOLCHAIN",
            "PATH",
            "HOME",
        ] {
            assert!(!is_cargo_injected(kept), "{kept} should be kept");
        }
    }
}
