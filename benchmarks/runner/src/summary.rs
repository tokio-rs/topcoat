//! The terminal summary printed at the end of a run.

use console::{measure_text_width, style};

use crate::{
    results::{AppResult, AppStatus, Results, Throughput},
    units,
};

/// Prints requests per second per app and scenario, then one line of resource figures per app.
pub fn print(results: &Results) {
    let primary = results.config.connections.first().copied().unwrap_or(0);
    let scenario_ids: Vec<&str> = results.scenarios.iter().map(|s| s.id.as_str()).collect();

    println!(
        "\n{} at {primary} connections (median run; {} = has errors, {} = exceeded the thread limit)",
        style("Requests per second").bold(),
        style("ERR").red(),
        style("LIMIT").yellow()
    );
    let mut header = vec!["app".to_owned(), "rendering".to_owned()];
    header.extend(scenario_ids.iter().map(|id| (*id).to_owned()));
    let mut rows = vec![header];
    for app in &results.apps {
        let mut row = vec![app_label(app), app.rendering.to_string()];
        if app.status == AppStatus::Ok || !app.scenarios.is_empty() {
            for id in &scenario_ids {
                row.push(rps_cell(primary_throughput(app, id, primary)));
            }
        } else {
            row.push(style(status_text(app)).yellow().to_string());
        }
        rows.push(row);
    }
    print_table(&rows);

    println!("\n{}", style("Resources").bold());
    let mut rows = vec![
        [
            "app",
            "startup",
            "idle mem",
            "peak mem",
            "req/cpu-s",
            "home br",
            "detail br",
        ]
        .map(str::to_owned)
        .to_vec(),
    ];
    for app in results.apps.iter().filter(|app| app.startup_ms.is_some()) {
        let medians: Vec<_> = app
            .scenarios
            .iter()
            .filter_map(|s| s.throughput.iter().find(|t| t.connections == primary))
            .filter(|t| t.valid && !t.exceeded_thread_limit)
            .filter_map(Throughput::median)
            .collect();
        let peak = medians.iter().map(|m| m.resources.peak_memory_bytes).max();
        let mut efficiency: Vec<f64> = medians
            .iter()
            .filter_map(|m| m.resources.requests_per_cpu_second)
            .collect();
        efficiency.sort_by(f64::total_cmp);
        let median_efficiency = efficiency
            .get(efficiency.len().saturating_sub(1) / 2)
            .copied();
        let payload = |page: &str| {
            app.payload
                .iter()
                .find(|p| p.page == page)
                .map_or_else(|| "-".to_owned(), |p| units::bytes(p.total.brotli))
        };
        rows.push(vec![
            app_label(app),
            app.startup_ms.map_or_else(|| "-".to_owned(), units::millis),
            app.idle_memory_bytes
                .map_or_else(|| "-".to_owned(), units::bytes),
            peak.map_or_else(|| "-".to_owned(), units::bytes),
            median_efficiency.map_or_else(|| "-".to_owned(), units::count),
            payload("home"),
            payload("detail"),
        ]);
    }
    print_table(&rows);
    println!(
        "{}",
        style("req/cpu-s: median over scenarios of successful requests per CPU-second of the app's process tree.").dim()
    );
}

fn app_label(app: &AppResult) -> String {
    if app.id == "topcoat" {
        style(&app.name).bold().to_string()
    } else {
        app.name.clone()
    }
}

fn status_text(app: &AppResult) -> String {
    let status = match app.status {
        AppStatus::Ok => "ok",
        AppStatus::Skipped => "skipped",
        AppStatus::BuildFailed => "build failed",
        AppStatus::StartFailed => "start failed",
        AppStatus::Crashed => "crashed",
        AppStatus::Interrupted => "interrupted",
    };
    match &app.error {
        Some(error) => format!("{status}: {}", error.lines().next().unwrap_or("")),
        None => status.to_owned(),
    }
}

fn primary_throughput<'a>(
    app: &'a AppResult,
    scenario: &str,
    connections: u32,
) -> Option<&'a Throughput> {
    app.scenarios
        .iter()
        .find(|s| s.id == scenario)?
        .throughput
        .iter()
        .find(|t| t.connections == connections)
}

fn rps_cell(throughput: Option<&Throughput>) -> String {
    match throughput {
        None => "-".to_owned(),
        Some(t) if !t.valid => style("ERR").red().to_string(),
        Some(t) if t.exceeded_thread_limit => style("LIMIT").yellow().to_string(),
        Some(t) => t
            .median()
            .map_or_else(|| "-".to_owned(), |m| units::count(m.rps)),
    }
}

fn print_table(rows: &[Vec<String>]) {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<usize> = (0..columns)
        .map(|column| {
            rows.iter()
                .filter(|row| row.len() == columns || column < row.len().saturating_sub(1))
                .filter_map(|row| row.get(column))
                .map(|cell| measure_text_width(cell))
                .max()
                .unwrap_or(0)
        })
        .collect();
    for (index, row) in rows.iter().enumerate() {
        let mut line = String::new();
        for (column, cell) in row.iter().enumerate() {
            let width = widths[column];
            let pad = width.saturating_sub(measure_text_width(cell));
            if column < 2 || row.len() < columns {
                line.push_str(cell);
                if column + 1 < row.len() {
                    line.push_str(&" ".repeat(pad));
                }
            } else {
                line.push_str(&" ".repeat(pad));
                line.push_str(cell);
            }
            line.push_str("  ");
        }
        let line = line.trim_end();
        if index == 0 {
            println!("{}", style(line).dim());
        } else {
            println!("{line}");
        }
    }
}
