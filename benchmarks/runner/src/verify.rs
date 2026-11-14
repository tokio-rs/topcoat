//! The `verify` command: the parity check from `SPEC.md`.

use std::{path::Path, process::ExitCode, time::Duration};

use anyhow::{Context, Result, bail};
use console::style;
use serde_json::Value;
use similar::{Algorithm, DiffTag, capture_diff_slices, group_diff_ops};

use crate::{
    config::Config,
    dataset::Dataset,
    html::Page,
    manifest::{Manifest, REFERENCE_APP},
    run::warn,
    scenario::Scenario,
    server::{self, Server},
};

/// How a response is compared with the reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Compare {
    Text,
    Html,
    Json,
    /// A `422` body: an object with a string `error`. The message itself may differ.
    JsonError,
    /// Only the status code, for `404` responses.
    Status,
}

struct Check {
    name: String,
    method: &'static str,
    path: String,
    body: Option<String>,
    cookie: Option<String>,
    status: u16,
    compare: Compare,
}

#[derive(Clone)]
struct Response {
    status: u16,
    content_type: String,
    /// The `Set-Cookie` header, if any.
    set_cookie: Option<String>,
    body: String,
}

pub fn verify(root: &Path, config: &Config, verbose: bool) -> Result<ExitCode> {
    let manifests = Manifest::discover_selected(root, &config.apps)?;
    let Some(reference) = manifests.iter().find(|m| m.id == REFERENCE_APP) else {
        bail!(
            "the reference app `{REFERENCE_APP}` has no app.toml under {}",
            root.join("apps").display()
        );
    };
    let selected: Vec<&Manifest> = manifests
        .iter()
        .filter(|m| config.selects_app(&m.id))
        .collect();
    if selected.is_empty() {
        bail!("no apps selected");
    }
    server::install_interrupt_handler();

    let data_dir = root.join("data");
    let dataset = Dataset::load(&data_dir);
    let checks = checks(&dataset, config.cart_count);
    let logs = config.output_root(root).join("verify-logs");
    std::fs::create_dir_all(&logs)?;

    println!(
        "{} {} checks per app against {}",
        style("verify").bold(),
        checks.len(),
        reference.name
    );
    let reference_responses = collect(reference, config, &data_dir, &logs, &checks)
        .context("the reference app failed")?;
    let mut failures = 0;
    let mut skipped = Vec::new();
    for app in selected {
        if server::interrupted() {
            break;
        }
        let missing = app.missing_requirements();
        if !missing.is_empty() {
            warn(&format!(
                "skipping {}: missing commands: {}",
                app.id,
                missing.join(", ")
            ));
            skipped.push(app.id.clone());
            continue;
        }
        let responses = if app.id == reference.id {
            Ok(reference_responses.clone())
        } else {
            collect(app, config, &data_dir, &logs, &checks)
        };
        let responses = match responses {
            Ok(responses) => responses,
            Err(error) => {
                println!("{} {}: {error:#}", style("FAIL").red().bold(), app.name);
                failures += 1;
                continue;
            }
        };
        let mut problems = Vec::new();
        for (check, (expected, actual)) in checks
            .iter()
            .zip(reference_responses.0.iter().zip(&responses.0))
        {
            if let Some(problem) = compare(check, expected, actual, verbose) {
                problems.push((check, problem));
            }
        }
        if let Some(problem) = cookie_problem(&checks, &responses) {
            problems.push((&checks[0], problem));
        }
        let cookie_setters: Vec<(&Check, &str)> = checks
            .iter()
            .zip(&responses.0)
            .filter(|(check, _)| check.method == "GET" && check.compare == Compare::Html)
            .filter_map(|(check, response)| Some((check, response.set_cookie.as_deref()?)))
            .collect();
        if let Some((first, cookie)) = cookie_setters.first() {
            warn(&format!(
                "{}: {} HTML pages set a cookie (`GET {}` sent `Set-Cookie: {}`); the pages are stateless, so check that this is the framework's default behavior",
                app.id,
                cookie_setters.len(),
                first.path,
                cookie.split(';').next().unwrap_or("")
            ));
        }
        if problems.is_empty() {
            println!(
                "{} {} ({} checks)",
                style("ok").green().bold(),
                app.name,
                checks.len()
            );
        } else {
            failures += 1;
            println!(
                "{} {} ({} of {} checks failed)",
                style("FAIL").red().bold(),
                app.name,
                problems.len(),
                checks.len()
            );
            for (check, problem) in &problems {
                println!(
                    "  {} {} {}",
                    style("x").red(),
                    style(&check.name).bold(),
                    style(format!("{} {}", check.method, check.path)).dim()
                );
                for line in problem.lines() {
                    println!("      {line}");
                }
            }
        }
    }
    if !skipped.is_empty() {
        println!("skipped: {}", skipped.join(", "));
    }
    Ok(if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

#[derive(Clone)]
struct Responses(Vec<Response>);

fn checks(dataset: &Dataset, cart_count: u32) -> Vec<Check> {
    let cookie = Some(format!("cart_count={cart_count}"));
    let mut checks = Vec::new();
    let mut add = |name: String,
                   method,
                   path: String,
                   body: Option<String>,
                   cookie: Option<String>,
                   status,
                   compare| {
        checks.push(Check {
            name,
            method,
            path,
            body,
            cookie,
            status,
            compare,
        });
    };
    // The cookie checks come first; `cookie_problem` relies on their positions.
    add(
        "cart cookie".into(),
        "GET",
        "/".into(),
        None,
        Some("cart_count=7".into()),
        200,
        Compare::Html,
    );
    add(
        "no cart cookie".into(),
        "GET",
        "/".into(),
        None,
        None,
        200,
        Compare::Html,
    );
    add(
        "garbage cart cookie".into(),
        "GET",
        "/".into(),
        None,
        Some("cart_count=lots".into()),
        200,
        Compare::Html,
    );

    for scenario in Scenario::all(dataset) {
        let compare = match scenario.response {
            crate::scenario::Body::Text => Compare::Text,
            crate::scenario::Body::Html => Compare::Html,
            crate::scenario::Body::Json => Compare::Json,
        };
        let paths: Vec<String> = if scenario.paths.len() > 8 {
            sample_ids(dataset)
                .into_iter()
                .map(|id| {
                    let prefix = scenario.paths[0]
                        .rsplit_once('/')
                        .map_or("", |(prefix, _)| prefix);
                    format!("{prefix}/{id}")
                })
                .collect()
        } else {
            scenario.paths.clone()
        };
        if scenario.bodies.is_empty() {
            for path in paths {
                add(
                    scenario.id.to_owned(),
                    "GET",
                    path,
                    None,
                    cookie.clone(),
                    200,
                    compare,
                );
            }
        } else {
            for body in scenario
                .bodies
                .iter()
                .step_by((scenario.bodies.len() / 4).max(1))
                .take(4)
            {
                add(
                    scenario.id.to_owned(),
                    "POST",
                    scenario.paths[0].clone(),
                    Some(body.clone()),
                    cookie.clone(),
                    200,
                    compare,
                );
            }
        }
    }

    let extra_queries = [
        "",
        "?sort=price-desc",
        "?sort=rating&page=2",
        "?sort=newest&page=10",
    ];
    let category = dataset
        .categories
        .get(1)
        .cloned()
        .unwrap_or_else(|| "outdoor".to_owned());
    for query in extra_queries
        .iter()
        .map(|q| (*q).to_owned())
        .chain([format!("?category={category}&sort=rating")])
    {
        add(
            "listing variant".into(),
            "GET",
            format!("/products{query}"),
            None,
            cookie.clone(),
            200,
            Compare::Html,
        );
        add(
            "api-list variant".into(),
            "GET",
            format!("/api/products{query}"),
            None,
            cookie.clone(),
            200,
            Compare::Json,
        );
    }

    add(
        "unknown product".into(),
        "GET",
        "/products/999999".into(),
        None,
        cookie.clone(),
        404,
        Compare::Status,
    );
    add(
        "unknown product (API)".into(),
        "GET",
        "/api/products/999999".into(),
        None,
        cookie.clone(),
        404,
        Compare::Status,
    );
    let first = dataset.in_stock;
    let mut invalid = vec![
        (
            "quantity 0",
            format!(r#"{{"product_id":{first},"quantity":0}}"#),
        ),
        (
            "quantity 11",
            format!(r#"{{"product_id":{first},"quantity":11}}"#),
        ),
        (
            "unknown product",
            r#"{"product_id":999999,"quantity":1}"#.to_owned(),
        ),
    ];
    if let Some(sold_out) = dataset.sold_out {
        invalid.push((
            "sold out",
            format!(r#"{{"product_id":{sold_out},"quantity":1}}"#),
        ));
    }
    // Bodies that are not a cart line at all must also be rejected as invalid input.
    invalid.push(("JSON array body", "[1, 2]".to_owned()));
    invalid.push(("malformed JSON", r#"{"product_id": 1,"#.to_owned()));
    for (name, body) in invalid {
        add(
            format!("cart: {name}"),
            "POST",
            "/api/cart".into(),
            Some(body),
            cookie.clone(),
            422,
            Compare::JsonError,
        );
    }
    checks
}

/// A spread of product ids including the edge cases the dataset has.
fn sample_ids(dataset: &Dataset) -> Vec<u32> {
    let ids = &dataset.product_ids;
    let mut sample: Vec<u32> = [
        0,
        ids.len() / 4,
        ids.len() / 2,
        ids.len() * 3 / 4,
        ids.len() - 1,
    ]
    .iter()
    .map(|index| ids[*index])
    .chain(
        [Some(dataset.in_stock), dataset.sold_out, dataset.on_sale]
            .into_iter()
            .flatten(),
    )
    .collect();
    sample.sort_unstable();
    sample.dedup();
    sample
}

fn collect(
    app: &Manifest,
    config: &Config,
    data_dir: &Path,
    logs: &Path,
    checks: &[Check],
) -> Result<Responses> {
    if config.build {
        println!("{} {}", style("build").cyan().bold(), app.name);
        server::build(
            app,
            config,
            data_dir,
            &logs.join(format!("{}.build.log", app.id)),
        )?;
    }
    let server = Server::start(app, config, data_dir, &logs.join(format!("{}.log", app.id)))?;
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .new_agent();
    let mut responses = Vec::new();
    for check in checks {
        let url = format!("{}{}", server.base_url(), check.path);
        let cookie = check.cookie.as_deref().unwrap_or("");
        let result = if let Some(body) = &check.body {
            agent
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Cookie", cookie)
                .send(body.as_bytes())
        } else {
            agent.get(&url).header("Cookie", cookie).call()
        };
        let mut response =
            result.with_context(|| format!("{} {} failed", check.method, check.path))?;
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_owned();
        let set_cookie = response
            .headers()
            .get("set-cookie")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        responses.push(Response {
            status: response.status().as_u16(),
            content_type,
            set_cookie,
            body: response
                .body_mut()
                .with_config()
                .limit(64 * 1024 * 1024)
                .read_to_string()?,
        });
    }
    server.stop();
    Ok(Responses(responses))
}

/// Returns a description of what differs, or `None` if the response matches.
fn compare(check: &Check, expected: &Response, actual: &Response, verbose: bool) -> Option<String> {
    if actual.status != check.status {
        return Some(format!(
            "status {} (expected {}, reference sent {}){}",
            actual.status,
            check.status,
            expected.status,
            body_excerpt(&actual.body)
        ));
    }
    if expected.status != check.status {
        return Some(format!(
            "the reference app sent status {} (expected {})",
            expected.status, check.status
        ));
    }
    if check.compare == Compare::Status {
        return None;
    }
    if let Some(problem) = content_type_problem(check.compare, &actual.content_type) {
        return Some(problem);
    }
    match check.compare {
        Compare::Status => None,
        Compare::Text => (expected.body.trim() != actual.body.trim()).then(|| {
            format!(
                "body {:?}, reference {:?}",
                actual.body.trim(),
                expected.body.trim()
            )
        }),
        Compare::JsonError => match serde_json::from_str::<Value>(&actual.body) {
            Ok(Value::Object(map)) if map.get("error").is_some_and(Value::is_string) => None,
            _ => Some(format!(
                "expected a JSON object with a string `error`{}",
                body_excerpt(&actual.body)
            )),
        },
        Compare::Json => {
            let parse = |text: &str| serde_json::from_str::<Value>(text);
            match (parse(&expected.body), parse(&actual.body)) {
                (Ok(expected), Ok(actual)) => {
                    let mut differences = Vec::new();
                    json_diff("", &expected, &actual, &mut differences);
                    (!differences.is_empty()).then(|| limit(differences, verbose).join("\n"))
                }
                (_, Err(error)) => Some(format!(
                    "invalid JSON: {error}{}",
                    body_excerpt(&actual.body)
                )),
                (Err(error), _) => Some(format!("the reference sent invalid JSON: {error}")),
            }
        }
        Compare::Html => html_problem(&expected.body, &actual.body, verbose),
    }
}

fn content_type_problem(compare: Compare, content_type: &str) -> Option<String> {
    let lower = content_type.to_ascii_lowercase();
    let mut parts = lower.split(';').map(str::trim);
    let essence = parts.next().unwrap_or("");
    let params: Vec<&str> = parts.map(|p| p.trim_matches(' ')).collect();
    let (want, charset_required) = match compare {
        Compare::Html => ("text/html", true),
        Compare::Text => ("text/plain", false),
        Compare::Json | Compare::JsonError => ("application/json", false),
        Compare::Status => return None,
    };
    let utf8 = params.iter().any(|p| p.replace('"', "") == "charset=utf-8");
    let other_params = params
        .iter()
        .any(|p| !p.is_empty() && p.replace('"', "") != "charset=utf-8");
    if essence != want || (charset_required && !utf8) || other_params {
        let expected = if charset_required {
            format!("{want}; charset=utf-8")
        } else {
            want.to_owned()
        };
        return Some(format!(
            "content type `{content_type}`, expected `{expected}`"
        ));
    }
    None
}

fn html_problem(expected: &str, actual: &str, verbose: bool) -> Option<String> {
    let expected = Page::parse(expected);
    let actual = Page::parse(actual);
    let mut out = Vec::new();
    let expected_counts = expected.element_counts();
    let actual_counts = actual.element_counts();
    for (name, count) in &expected_counts {
        let got = actual_counts.get(name).copied().unwrap_or(0);
        if got != *count {
            out.push(format!("<{name}> count {got}, reference {count}"));
        }
    }
    let expected_lines = expected.text_lines();
    let actual_lines = actual.text_lines();
    let key = |line: &String| {
        line.chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
    };
    let expected_keys: Vec<String> = expected_lines.iter().map(key).collect();
    let actual_keys: Vec<String> = actual_lines.iter().map(key).collect();
    if expected_keys.concat() != actual_keys.concat() {
        out.push("visible text differs (- reference, + app):".to_owned());
        let ops = capture_diff_slices(Algorithm::Myers, &expected_keys, &actual_keys);
        for group in group_diff_ops(ops, 1) {
            out.push(style("@@").dim().to_string());
            for op in group {
                let (tag, old, new) = op.as_tag_tuple();
                match tag {
                    DiffTag::Equal => out.extend(old.map(|i| format!("  {}", expected_lines[i]))),
                    DiffTag::Delete | DiffTag::Replace | DiffTag::Insert => {
                        out.extend(
                            old.map(|i| {
                                style(format!("- {}", expected_lines[i])).red().to_string()
                            }),
                        );
                        out.extend(
                            new.map(|i| {
                                style(format!("+ {}", actual_lines[i])).green().to_string()
                            }),
                        );
                    }
                }
            }
        }
    }
    (!out.is_empty()).then(|| limit(out, verbose).join("\n"))
}

/// Records the JSON paths where `actual` differs from `expected`. Numbers compare by value,
/// so `4` equals `4.0`.
fn json_diff(path: &str, expected: &Value, actual: &Value, out: &mut Vec<String>) {
    match (expected, actual) {
        (Value::Object(a), Value::Object(b)) => {
            for (key, value) in a {
                let child = format!("{path}/{key}");
                match b.get(key) {
                    Some(other) => json_diff(&child, value, other, out),
                    None => out.push(format!("{child}: missing")),
                }
            }
            for key in b.keys().filter(|key| !a.contains_key(*key)) {
                out.push(format!("{path}/{key}: unexpected field"));
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                out.push(format!(
                    "{}: {} items, reference {}",
                    display_path(path),
                    b.len(),
                    a.len()
                ));
            }
            for (index, (x, y)) in a.iter().zip(b).enumerate() {
                json_diff(&format!("{path}/{index}"), x, y, out);
            }
        }
        (Value::Number(a), Value::Number(b)) => {
            let same = match (a.as_i64(), b.as_i64(), a.as_u64(), b.as_u64()) {
                (Some(x), Some(y), _, _) => x == y,
                (_, _, Some(x), Some(y)) => x == y,
                _ => a.as_f64() == b.as_f64(),
            };
            if !same {
                out.push(format!("{}: {b}, reference {a}", display_path(path)));
            }
        }
        _ if expected != actual => {
            out.push(format!(
                "{}: {}, reference {}",
                display_path(path),
                short(actual),
                short(expected)
            ));
        }
        _ => {}
    }
}

fn display_path(path: &str) -> &str {
    if path.is_empty() { "/" } else { path }
}

fn short(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 120 {
        format!("{}...", text.chars().take(120).collect::<String>())
    } else {
        text
    }
}

/// Checks that the app shows the cart cookie: the page with `cart_count=7` must differ from
/// the page without a cookie.
fn cookie_problem(checks: &[Check], responses: &Responses) -> Option<String> {
    let text = |index: usize| -> Option<Vec<String>> {
        if checks.get(index)?.compare != Compare::Html {
            return None;
        }
        Some(Page::parse(&responses.0.get(index)?.body).text_lines())
    };
    let with_cookie = text(0)?;
    let without = text(1)?;
    (with_cookie == without).then(|| {
        "the page is identical with and without `cart_count=7`: the cart cookie is ignored"
            .to_owned()
    })
}

fn body_excerpt(body: &str) -> String {
    let trimmed: String = body.trim().chars().take(300).collect();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("\nbody: {trimmed}")
    }
}

fn limit(mut lines: Vec<String>, verbose: bool) -> Vec<String> {
    const MAX: usize = 40;
    if !verbose && lines.len() > MAX {
        let hidden = lines.len() - MAX;
        lines.truncate(MAX);
        lines.push(format!("... {hidden} more lines (use --verbose)"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_numbers_compare_by_value() {
        let mut out = Vec::new();
        json_diff(
            "",
            &serde_json::json!({"rating": 4.0}),
            &serde_json::json!({"rating": 4}),
            &mut out,
        );
        assert!(out.is_empty(), "{out:?}");
        json_diff(
            "",
            &serde_json::json!({"rating": 4.5}),
            &serde_json::json!({"rating": 4}),
            &mut out,
        );
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn json_diff_reports_missing_and_extra_fields() {
        let mut out = Vec::new();
        json_diff(
            "",
            &serde_json::json!({"a": 1, "b": 2}),
            &serde_json::json!({"a": 1, "c": 2}),
            &mut out,
        );
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn content_types_allow_utf8_charset_only() {
        assert!(content_type_problem(Compare::Html, "text/html; charset=utf-8").is_none());
        assert!(content_type_problem(Compare::Html, "text/html;charset=UTF-8").is_none());
        assert!(content_type_problem(Compare::Html, "text/html").is_some());
        assert!(content_type_problem(Compare::Json, "application/json").is_none());
        assert!(content_type_problem(Compare::Json, "application/json; charset=utf-8").is_none());
        assert!(content_type_problem(Compare::Json, "text/json").is_some());
    }
}
