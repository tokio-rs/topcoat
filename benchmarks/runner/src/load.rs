//! Load generation with `oha`.

use std::{
    collections::BTreeMap,
    path::Path,
    process::{Command, Stdio},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::{
    results::{Latency, Measurement},
    scenario::Scenario,
    units::Dur,
};

/// The `Accept-Encoding` a current Chrome sends, used when compression is on.
pub const ACCEPT_ENCODING: &str = "gzip, deflate, br, zstd";

/// One `oha` invocation.
#[derive(Clone, Copy)]
pub struct Load<'a> {
    pub base_url: &'a str,
    pub scenario: &'a Scenario,
    pub connections: u32,
    pub duration: Dur,
    /// Requests per second for an open-loop run; `None` for a closed-loop run.
    pub rate: Option<u64>,
    pub compression: bool,
    pub cart_count: u32,
    pub timeout: Dur,
    /// Directory for the URL and body files `oha` reads.
    pub work_dir: &'a Path,
}

impl Load<'_> {
    /// Runs `oha` and returns the measurement without resource usage.
    pub fn run(&self) -> Result<Measurement> {
        let mut command = Command::new("oha");
        command
            .args(["--no-tui", "--output-format", "json"])
            .args(["-z", &self.duration.oha_arg()])
            .args(["-c", &self.connections.to_string()])
            .args(["-t", &self.timeout.oha_arg()])
            // Let in-flight requests finish at the deadline instead of counting them as
            // aborted, and never follow redirects: every benchmarked route answers directly.
            .arg("-w")
            .args(["-r", "0"])
            .args(["-H", &format!("Cookie: cart_count={}", self.cart_count)])
            // Without this `oha` sends its own `Accept-Encoding` and decompresses responses.
            .arg("--disable-compression");
        if self.compression {
            command.args(["-H", &format!("Accept-Encoding: {ACCEPT_ENCODING}")]);
        }
        if let Some(rate) = self.rate {
            command
                .args(["-q", &rate.to_string()])
                .arg("--latency-correction");
        }
        if self.scenario.method != "GET" {
            command.args(["-m", self.scenario.method]);
        }
        if !self.scenario.bodies.is_empty() {
            let bodies = self.work_dir.join(format!("{}.bodies", self.scenario.id));
            std::fs::write(&bodies, self.scenario.bodies.join("\n") + "\n")?;
            command
                .args(["-T", "application/json"])
                .arg("-Z")
                .arg(bodies);
        }
        if let [path] = self.scenario.paths.as_slice() {
            command.arg(format!("{}{path}", self.base_url));
        } else {
            let mut urls = String::new();
            for path in &self.scenario.paths {
                urls.push_str(self.base_url);
                urls.push_str(path);
                urls.push('\n');
            }
            let file = self.work_dir.join(format!("{}.urls", self.scenario.id));
            std::fs::write(&file, urls)?;
            command.arg("--urls-from-file").arg(file);
        }

        let output = command
            .stdin(Stdio::null())
            .output()
            .context("cannot run `oha`, is it installed?")?;
        if !output.status.success() {
            bail!(
                "oha failed with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        let report: OhaReport = serde_json::from_slice(&output.stdout).with_context(|| {
            format!(
                "cannot parse oha output: {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })?;
        Ok(report.into_measurement())
    }
}

/// The parts of `oha --output-format json` the runner reads. Times are in seconds.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OhaReport {
    summary: OhaSummary,
    latency_percentiles: Option<BTreeMap<String, Option<f64>>>,
    #[serde(default)]
    status_code_distribution: BTreeMap<String, u64>,
    #[serde(default)]
    error_distribution: BTreeMap<String, u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OhaSummary {
    total: f64,
    slowest: Option<f64>,
    fastest: Option<f64>,
    average: Option<f64>,
    total_data: Option<u64>,
}

impl OhaReport {
    #[expect(clippy::cast_precision_loss, reason = "request counts fit in f64")]
    fn into_measurement(self) -> Measurement {
        let ms = |secs: Option<f64>| {
            secs.filter(|value| value.is_finite())
                .map(|value| value * 1000.0)
        };
        let percentile = |name: &str| {
            ms(self
                .latency_percentiles
                .as_ref()
                .and_then(|map| map.get(name).copied().flatten()))
        };

        let mut successful = 0;
        let mut failed_statuses = Vec::new();
        let mut redirects = 0;
        for (code, count) in &self.status_code_distribution {
            match code.parse::<u16>() {
                Ok(200..=299) => successful += count,
                Ok(300..=399) => {
                    successful += count;
                    redirects += count;
                }
                _ => failed_statuses.push(format!("{count} responses with status {code}")),
            }
        }
        let responses: u64 = self.status_code_distribution.values().sum();
        let error_count: u64 = self.error_distribution.values().sum();
        let requests = responses + error_count;

        let mut problems = failed_statuses;
        for (error, count) in &self.error_distribution {
            problems.push(format!("{count} connection errors: {error}"));
        }
        if successful == 0 {
            problems.push("no successful responses".to_owned());
        }
        let valid = problems.is_empty();
        if redirects > 0 {
            problems.push(format!(
                "{redirects} redirect responses (counted as successful)"
            ));
        }

        let duration = self.summary.total;
        let bytes = self.summary.total_data.unwrap_or(0);
        Measurement {
            valid,
            exceeded_thread_limit: false,
            problems,
            duration_secs: duration,
            requests,
            successful_requests: successful,
            rps: if duration > 0.0 {
                successful as f64 / duration
            } else {
                0.0
            },
            latency_ms: Latency {
                mean: ms(self.summary.average),
                min: ms(self.summary.fastest),
                p50: percentile("p50"),
                p90: percentile("p90"),
                p99: percentile("p99"),
                p999: percentile("p99.9"),
                max: ms(self.summary.slowest),
            },
            response_bytes: bytes,
            bytes_per_response: if responses > 0 {
                bytes as f64 / responses as f64
            } else {
                0.0
            },
            transfer_bytes_per_sec: if duration > 0.0 {
                bytes as f64 / duration
            } else {
                0.0
            },
            status_codes: self.status_code_distribution,
            errors: self.error_distribution,
            error_count,
            resources: crate::results::Resources::default(),
        }
    }
}

/// Sends one request of `scenario` with the browser `Accept-Encoding` and returns the
/// `Content-Encoding` of the response, or `identity` if it has none.
pub fn probe_content_encoding(port: u16, scenario: &Scenario, cart_count: u32) -> Option<String> {
    use std::{
        fmt::Write as _,
        io::{Read, Write},
    };

    let path = scenario.paths.first()?;
    let body = scenario.bodies.first().map_or("", String::as_str);
    let mut request = format!(
        "{} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept-Encoding: {ACCEPT_ENCODING}\r\nCookie: cart_count={cart_count}\r\nConnection: close\r\n",
        scenario.method
    );
    if !body.is_empty() {
        let _ = write!(
            request,
            "Content-Type: application/json\r\nContent-Length: {}\r\n",
            body.len()
        );
    }
    request.push_str("\r\n");
    request.push_str(body);

    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .ok()?;
    stream.write_all(request.as_bytes()).ok()?;
    let mut head = Vec::new();
    let mut buffer = [0u8; 4096];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") && head.len() < 64 * 1024 {
        let read = stream.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        head.extend_from_slice(&buffer[..read]);
    }
    let head = String::from_utf8_lossy(&head);
    let encoding = head
        .split("\r\n\r\n")
        .next()?
        .lines()
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-encoding"))
        .map_or_else(
            || "identity".to_owned(),
            |(_, value)| value.trim().to_owned(),
        );
    Some(encoding)
}

/// The installed `oha` version, e.g. `oha 1.14.0`.
pub fn oha_version() -> Option<String> {
    let output = Command::new("oha").arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(statuses: &[(&str, u64)], errors: &[(&str, u64)]) -> Measurement {
        let json = serde_json::json!({
            "summary": { "total": 2.0, "slowest": 0.01, "fastest": 0.001, "average": 0.002, "totalData": 1000 },
            "latencyPercentiles": { "p50": 0.002, "p90": 0.003, "p99": 0.004, "p99.9": 0.005 },
            "statusCodeDistribution": statuses.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect::<BTreeMap<_, _>>(),
            "errorDistribution": errors.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect::<BTreeMap<_, _>>(),
        });
        serde_json::from_value::<OhaReport>(json)
            .unwrap()
            .into_measurement()
    }

    #[test]
    fn fast_error_responses_invalidate_the_measurement() {
        let measurement = report(&[("200", 10), ("500", 90_000)], &[]);
        assert!(!measurement.valid);
        // Throughput only counts successful responses.
        assert!((measurement.rps - 5.0).abs() < 1e-9);
    }

    #[test]
    fn connection_errors_invalidate_the_measurement() {
        assert!(!report(&[("200", 100)], &[("connection refused", 1)]).valid);
        assert!(!report(&[], &[]).valid);
        assert!(report(&[("200", 100)], &[]).valid);
    }

    #[test]
    fn latency_is_reported_in_milliseconds() {
        let measurement = report(&[("200", 100)], &[]);
        assert!((measurement.latency_ms.p99.unwrap() - 4.0).abs() < 1e-9);
    }
}
