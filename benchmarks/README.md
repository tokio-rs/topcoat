# Web framework benchmarks

This directory compares Topcoat with other web frameworks on one realistic app: "Harbor Market", a small storefront with a home page, product listing and detail pages, a large orders table, a JSON API and two small interactive pieces. Every framework implements the same app under `apps/<id>/` following [`SPEC.md`](SPEC.md), and a Rust runner builds, starts and load tests each one in turn, then writes the numbers to `results.json` and a self-contained HTML report.

## Quick start

With Nix, everything the suite needs (the Rust toolchain with the WebAssembly target, oha, Node.js, Ruby, .NET, Go, Python, Elixir, Tailwind CSS and the Leptos and Dioxus CLIs) comes pinned from the repository's `flake.lock`:

```sh
nix develop .#benchmarks
cargo bench-web doctor                 # what is installed, which apps are ready
cargo bench-web run --profile quick    # every app, every scenario, a few minutes
open benchmarks/results/latest/report.html
```

Without Nix, the runner uses whatever is on `PATH`. It needs [oha](https://github.com/hatoo/oha) for load generation; each app lists the commands it needs in its manifest, and apps whose commands are missing are skipped with a warning. `cargo bench-web doctor` shows what is missing.

```sh
cargo install oha
cargo bench-web run --profile quick --only topcoat,axum-maud
```

`cargo bench-web` is a cargo alias for `cargo run --release -p bench-runner --bin bench-web --`, so it works from anywhere in the repository.

## Commands

| Command | What it does |
|---------|--------------|
| `run` | Builds the selected apps, then benchmarks them one at a time and writes `results/<timestamp>-<profile>/` with `results.json`, `report.html` and the app logs. Prints a summary table at the end. Exits with 2 if any measurement was invalid. |
| `verify` | Checks every selected app against the reference app `axum-maud`: status codes, content types, visible text, element counts and JSON. Prints a diff for each mismatch. See [Parity check](SPEC.md#parity-check). |
| `report <dir>` | Regenerates `report.html` from a results directory or a `results.json` file, for example after updating the runner. |
| `list` | Lists the apps, the scenarios and the profiles. |
| `doctor` | Shows the load generator, dataset, configuration, each app's readiness and the toolchain versions. |

A run goes like this. All selected apps are built first, so builds never overlap a measurement. Then, for each app: start it on a free port in its own process group, wait for `GET /health` to answer, record the startup time and the idle memory, and for each scenario and connection count run a warmup followed by the measured runs while sampling the memory and CPU time of the app's process tree. An optional open-loop latency phase follows each scenario. Finally the runner measures the client payload, stops the app's whole process group and cools down before the next app. Ctrl-C stops the running app and writes partial results; a second Ctrl-C quits immediately.

## Configuration

[`bench.toml`](bench.toml) holds every option with its default and a comment explaining it. Values resolve in this order, later ones winning: built-in defaults, the top level of `bench.toml`, the selected profile, command line flags. `cargo bench-web run --dry-run` prints the resolved configuration and the time estimate without running anything.

### Profiles

| Profile | Warmup | Duration | Runs | Connections | Latency phase | Roughly |
|---------|--------|----------|------|-------------|---------------|---------|
| `quick` | 500ms | 1s | 1 | 32 | off | 20 seconds per app |
| `standard` (default) | 5s | 10s | 1 | 64 | off | 3 minutes per app |
| `full` | 10s | 15s | 3, median reported | 64, 16, 256 | 30s, on | 30 minutes per app |

Use `quick` while developing an app or the runner, `standard` for a meaningful comparison and `full` for published numbers. The plan printed at the start of a run includes a time estimate (builds not included).

### Options

| `bench.toml` key | Flag | Default | Meaning |
|------------------|------|---------|---------|
| `profile` | `-p`, `--profile` | `standard` | Named profile from `[profiles.<name>]`. |
| `apps` | `--only a,b` | all | Apps to run, by id. |
| `skip` | `--skip a,b` | none | Apps to leave out. |
| `scenarios` | `--scenarios a,b` | all | Scenarios to run. |
| `skip_scenarios` | `--skip-scenarios a,b` | none | Scenarios to leave out. |
| `warmup` | `-w`, `--warmup` | `5s` | Load before the measured runs of each scenario and connection count. |
| `duration` | `-d`, `--duration` | `10s` | Length of each measured run. |
| `runs` | `-r`, `--runs` | `1` | Measured runs per point; the median by throughput is reported. |
| `connections` | `-c`, `--connections 16,64` | `[64]` | Concurrent connections; one measurement per value. The first is the primary value for summaries. |
| `threads` | `--threads` | `0` | `BENCH_THREADS` for every app: worker threads, or worker processes for Node, Ruby and Python. `0` is the framework default (all cores). |
| `compression` | `--compression on\|off` | off | `BENCH_COMPRESSION` for every app. When on, the load generator sends `Accept-Encoding` and response sizes are compressed sizes. |
| `allocator` | `--allocator` | `jemalloc` | Global allocator of Rust apps: `system`, `jemalloc` or `mimalloc`, selected through cargo features. |
| `cooldown` | `--cooldown` | `2s` | Pause after each scenario and after each app. |
| `build` | `--skip-build` | build | Build the apps before running them. |
| `payload` | `--no-payload` | on | Measure the client payload. |
| `output_dir` | `-o`, `--output-dir` | `results` | Where result directories go, relative to `benchmarks/`. |
| `open` | `--open` | off | Open the report when done. |
| `sample_interval` | `--sample-interval` | `250ms` | Memory and CPU sampling interval. |
| `request_timeout` | `--request-timeout` | `10s` | A slower request counts as an error. |
| `idle_settle` | | `2s` | Wait after the first health check before reading idle memory. |
| `thread_limit_margin` | | `0.5` | With `threads` set to N, a measurement is flagged when the app kept more than `N * (1 + margin) + 0.25` cores busy. |
| `cart_count` | | `3` | The `cart_count` cookie every request sends. |
| `latency.enabled` | `--latency on\|off` | off | Run the open-loop latency phase. |
| `latency.mode` | `--latency-mode` | `relative` | `relative`: a fraction of the app's own measured throughput. `fixed`: the same rate for every app. |
| `latency.fraction` | `--latency-fraction` | `0.5` | Fraction used by `relative` mode. |
| `latency.rate` | `--latency-rate` | `1000` | Requests per second used by `fixed` mode. |
| `latency.rates` | | `{}` | Per-scenario rates for `fixed` mode. |
| `latency.duration` | `--latency-duration` | `10s` | Length of the latency phase. |
| `latency.connections` | `--latency-connections` | `64` | Connections available to the open-loop generator. |

The `[expected_tools]` table maps commands to the versions the apps are expected to be built with (for example `tailwindcss = "4.3.3"`). `run` and `doctor` warn when the tool on `PATH` reports another version, and `results.json` records the versions actually used.

`verify` accepts `--only`, `--skip`, `--skip-build`, `--allocator`, `--compression` and `--threads`. Every command accepts `--config <file>` to use another configuration file, and `BENCH_ROOT` points the runner at another `benchmarks/` directory.

Durations are written as `500ms`, `10s`, `2m` or `1h`.

## Scenarios

Every request carries the `cart_count` cookie, since every HTML page renders it.

| Scenario | Request | What it stresses |
|----------|---------|------------------|
| `plaintext` | `GET /health` | The framework floor: routing and a tiny response. |
| `about` | `GET /about` | A small, mostly static page in the shared layout. |
| `home` | `GET /` | Hero, category links and 8 product cards. |
| `listing` | `GET /products?...`, two queries | 24 product cards, a category sidebar with counts, sort links and pagination. |
| `detail` | `GET /products/{id}` over all 240 products | A product with its spec table, reviews and related products. |
| `orders` | `GET /admin/orders` | A large page: 500 table rows with formatted money and dates. |
| `api-list` | `GET /api/products?...`, two queries | JSON for the listing queries. |
| `api-detail` | `GET /api/products/{id}` over all 240 products | Full product JSON. |
| `api-cart` | `POST /api/cart` over valid bodies | JSON parsing, validation against the catalog and a JSON response. |

Requests that rotate pick a random path or body per request.

## Results

Each run writes `results/<timestamp>-<profile>/`, and `results/latest` points at the newest one. The directory is ignored by git.

- `report.html`: the report. It is a single file with the data embedded, works offline and can be shared as is. It covers an overview of every app and scenario, throughput, latency percentiles, memory, CPU efficiency, startup time, client payload, a sortable table of every measurement, the configuration, the environment and the methodology. It follows the system light or dark mode and has a toggle.
- `results.json`: everything measured, described below.
- `logs/<id>.build.log` and `logs/<id>.log`: build and server output.

### `results.json`

The schema is defined by the types in [`runner/src/results.rs`](runner/src/results.rs) and versioned by `schema_version`. Units are part of the field names (`_bytes`, `_ms`, `_secs`; `rps` is requests per second), and values that were not measured are `null`.

```text
schema_version, runner_version, started_at, finished_at, interrupted
config          the resolved configuration (every option above)
environment     os, os_version, kernel, arch, cpu, physical_cores, logical_cores, memory_bytes,
                memory_metric ("pss" or "rss"), git_commit, git_dirty, oha, nix_shell,
                toolchains { command: version line },
                tool_version_mismatches { command: "expected X, found Y" }
scenarios[]     id, title, description, method, response, sample_paths, path_count, body_count
apps[]
  id, name, language, kind, rendering, description, framework_version
  status        ok | skipped | build_failed | start_failed | crashed | interrupted
  error, build_secs, artifact_bytes (not comparable across runtimes), startup_ms,
  idle_memory_bytes, cpu_affinity (CPU list the app was pinned to, or null)
  scenarios[]
    id, content_encoding (negotiated Content-Encoding when compression is on)
    throughput[]  one per connection count
      connections, valid, exceeded_thread_limit, warmup_problems[], runs[], median_index,
      rps_min, rps_max
    latency       null, or the open-loop phase:
      mode, target_rps, connections, saturated, measurement
  payload[]     one per page ("home", "detail")
    page, path, assets[] { path, category, source, sizes }, totals { html|js|wasm|css|other: sizes },
    total, inline_script_bytes, inline_style_bytes, missing[]

measurement     valid, exceeded_thread_limit, problems[], duration_secs, requests, successful_requests, rps,
                latency_ms { mean, min, p50, p90, p99, p999, max },
                response_bytes, bytes_per_response, transfer_bytes_per_sec,
                status_codes { code: count }, errors { message: count }, error_count,
                resources { peak_memory_bytes, mean_memory_bytes, cpu_seconds, cpu_cores_used,
                            requests_per_cpu_second, samples }
sizes           raw, gzip, brotli
```

## Methodology

### Fairness rules

[`SPEC.md`](SPEC.md#goals-and-ground-rules) holds the full list. The rules that most affect the numbers:

- **No response caching** (rule 3). No full-page, fragment or HTTP caching and no static prerendering of the benchmarked routes; every request renders. Every page depends on the `cart_count` cookie anyway.
- **The data layer is not under test** (rule 4). No database: each app loads the shared JSON files at startup and does only O(1) lookups and slices per request.
- **Standard formatters** (rule 9). Dates, money and thousands separators use the ecosystem's standard formatter where one can express the output, even when a hand-written version would be faster.
- **Framework URL generation** (rule 10). Internal links come from the framework's own URL helpers where it has them.
- **Loading versus formatting** (rule 11). Parsing the dataset into typed values happens at startup; formatting always happens per request.
- **Default middleware stays** (rule 12). Each app keeps the middleware and response headers its framework's generator sets up, and lists every removal in its `README.md`.
- **`BENCH_THREADS=N` means at most N cores** (rule 13). On Linux the runner pins the app's process group to N CPUs with `taskset`; on every platform it flags any measurement whose process tree kept materially more than N cores busy as "exceeded thread limit, not comparable", and the report does not draw it.

### How fairness is enforced

- Every app implements the same pages with the same markup, styling and data, and must pass `verify` against the reference app. [`SPEC.md`](SPEC.md) sets the ground rules: idiomatic code, production settings, no response caching, no access logs, pinned versions.
- Apps run one at a time with the same environment and settings. Rust apps share one build profile and allocator setting. The variables cargo injects into processes it runs (the runner itself is started with `cargo run`) are removed from every app's environment.
- Any failed request (a status outside 2xx and 3xx, a connection error or a timeout) invalidates the whole measurement. Invalid measurements are reported with their errors and never drawn as bars, so an app that fails quickly cannot look fast.
- Memory and CPU time cover the app's whole process tree, so servers that fork worker processes (Node, Ruby, Python) are measured completely.
- Requests per CPU-second divides successful requests by the CPU time the app's process tree used. It compares efficiency independently of how many cores a server managed to keep busy.
- The client payload is computed by the runner: it fetches the HTML of the home and detail pages and every script, stylesheet, preload and WebAssembly module they reference (plus string-literal imports found in scripts and the manifest's `extra_assets`), uncompressed, and compresses everything itself with gzip level 9 and brotli quality 11. Apps are compared on what they ship, not on how their servers are configured. `<script nomodule>` is skipped because modern browsers never fetch it.
- With compression on, the load generator sends the `Accept-Encoding` of a current Chrome (`gzip, deflate, br, zstd`), so these runs compare each framework's default compression algorithm and level. The negotiated `Content-Encoding` is recorded per app and scenario and shown next to the numbers.
- Artifact sizes are recorded but not charted: the manifests count different things (some include the runtime or dependencies, some do not), so they are not comparable across runtimes.

### Load generation

Throughput runs are closed-loop: [oha](https://github.com/hatoo/oha) keeps a fixed number of keep-alive connections busy, each sending its next request as soon as the previous response arrives. This finds the maximum throughput. The latency reported for these runs mostly reflects queueing at full load.

The latency phase is open-loop: requests are scheduled at a fixed rate whether or not earlier ones finished, and each latency is measured from the scheduled send time. This corrects for coordinated omission, where a stalled server would otherwise also stall the load generator and hide its own slowness. The rate matters, and with throughput differing by up to two orders of magnitude between apps no single rate is fair to all of them:

- `relative` mode (default) runs each app at a fraction (50%) of the throughput it reached in the same run. Every app has the same headroom, so the comparison is about latency behavior: tail latency, garbage collection pauses, scheduling. The offered rate differs per app and is shown next to every latency figure.
- `fixed` mode sends the same rate to every app, which answers "what does a user see at this load". Fast apps are barely loaded, and apps that cannot reach the rate are flagged as saturated because their latency then only measures queue growth. Use `latency.rates` to pick a rate per scenario.

### Memory and CPU sampling

The runner samples the app's process tree every 250ms during each measured run, from a separate thread. Memory is the sum over all processes in the tree of the proportional set size (PSS) on Linux, which splits shared pages between the processes that map them, and of the resident set size (RSS) on macOS, where PSS is not available. RSS counts shared pages once per process, so on macOS servers with several worker processes look bigger than they are. Peak and mean memory are taken over the samples of a run. CPU time is the accumulated user plus system time of every process in the tree, measured at the start and the end of the run; processes that exit during the run keep the last time seen.

### What is not measured

- No database. The data layer is in memory by design, so the numbers isolate routing, rendering and serialization. Real applications often spend most of their time waiting on I/O, which narrows the differences shown here.
- No TLS, HTTP/2, reverse proxy or network latency. Everything runs over loopback.
- Nothing in the browser: no hydration time, time to interactive or browser memory. The client payload only counts bytes.

## Caveats

- **The frameworks differ in kind.** Next.js, Leptos and Dioxus render on the server and then hydrate a client runtime. The server does extra work for that, serializing state and emitting hydration markers, and the browser downloads a JavaScript or WebAssembly bundle and runs it before the page is interactive. Leptos in islands mode hydrates only the interactive parts and ships less. Topcoat, Axum with Maud, Rails, Django, Phoenix, ASP.NET Core and Go with templ render on the server and ship small or no client runtimes. Server throughput alone does not capture this trade-off: a hydrating framework does more per request and ships more to the browser, and in exchange gives a different programming model for interactive applications. The client payload section and the rendering model shown next to every name exist to keep that context visible.
- **The load generator shares the machine.** oha runs on the same machine as the server and competes with it for CPU. Very fast servers are partly limited by the load generator, so differences at the top end are compressed. Lowering `threads` leaves cores to the load generator at the cost of not using the whole machine.
- **Results depend on the machine.** Operating system, scheduler, thermal state and background load all matter. Compare numbers from the same run only, and prefer Linux for published numbers so memory is reported as PSS.
- **Process-per-core runtimes.** Node.js, Ruby and Python scale by running one worker process per core, while the other runtimes run threads in one process. All of them get the same number of cores (`BENCH_THREADS`), and requests per CPU-second is the normalized view that does not depend on how well a server fills the machine.
- **Memory on macOS is RSS**, which counts shared pages once per process and so overstates servers that fork workers. Linux reports PSS and is the recommended platform for published numbers.
- **Short profiles are noisy.** `quick` is for smoke tests; JIT-compiled runtimes (Node.js, .NET, Ruby with YJIT, the BEAM) need longer warmups than it gives them.

### Per-framework notes

These are known differences between the apps, each kept for the stated reason:

- **Next.js** runs with Cache Components and partial prerendering off. The suite forbids prerendering the benchmarked routes for every framework (rule 3); a prerendered variant would measure something else and would be a different benchmark.
- **Rails** formats money with `number_to_currency`, which the app's author measured at roughly 3.5 times the cost of a hand-written formatter on the orders page. Rule 9 requires each ecosystem's standard helper, so it stays.
- **Middleware.** ASP.NET Core, Django, Rails and Phoenix keep the default middleware stacks their generators set up (rule 12). Axum and Go's `net/http` have no default middleware, so those apps run with almost none.
- **Dioxus** cannot honor a thread limit: in 0.7 it renders pages on its own pool of one thread per core, which is not configurable. With `threads` set, its page scenarios are flagged as exceeding the thread limit.
- **Django** runs Gunicorn with `gthread` workers, one thread per worker. Sync workers would be the simplest choice, but they cannot keep connections alive without a reverse proxy in front, and every app here is measured over keep-alive connections.
- **Phoenix** on macOS appeared to keep only about 3 cores busy at default settings in early runs taken while other builds were loading the machine. A `standard` run on a quiet machine did not reproduce this: Phoenix used 6.6 to 7.0 of 8 cores on the HTML scenarios, in line with the other apps.

## Frameworks

| Id | Framework | Rendering | Stack | Server and workers | `BENCH_THREADS=N` | `BENCH_COMPRESSION=on` | Client-side code |
|----|-----------|-----------|-------|--------------------|-------------------|------------------------|------------------|
| `topcoat` | Topcoat 0.12 (this repository) | ssr | Rust: `view!` components, module routing, Tailwind and assets bundled by the Topcoat CLI | Topcoat's hyper server on a Tokio multi-threaded runtime | `worker_threads(N)` | Topcoat's default compression (gzip and brotli, quality 4); `off` disables it | Topcoat's browser runtime, about 48 KB minified |
| `axum-maud` | Axum 0.8.9 + Maud 0.27 (reference) | ssr | Rust: Axum routing, Maud compile-time templates | Axum on a Tokio multi-threaded runtime | `worker_threads(N)` | tower-http `CompressionLayer` defaults | Plain JS, about 1.9 KB |
| `leptos` | Leptos 0.8.23 | ssr+hydration | Rust: `leptos_axum`, `leptos_router`, `SsrMode::Async` with server functions | Axum on a Tokio multi-threaded runtime | `worker_threads(N)` | tower-http `CompressionLayer` defaults | Whole-app hydration, WebAssembly bundle |
| `leptos-islands` | Leptos 0.8.23, islands | islands | The `leptos` app with the `islands` feature and two `#[island]`s | Axum on a Tokio multi-threaded runtime | `worker_threads(N)` | tower-http `CompressionLayer` defaults | Two islands, small WebAssembly bundle |
| `dioxus` | Dioxus 0.7.10 fullstack | ssr+hydration | Rust: `rsx!` components, Dioxus router, server functions, built with `dx bundle` | Axum on Tokio, pages rendered on Dioxus's own pool of one thread per core | `worker_threads(N)` for Tokio; the render pool ignores it | tower-http `CompressionLayer` defaults | Whole-app hydration, WebAssembly bundle |
| `nextjs` | Next.js 16.4.6, React 19.3 | rsc+hydration | TypeScript: App Router, React Server Components, standalone output | Node.js 24, one process per core via `node:cluster` | N worker processes | Next.js built-in gzip (`compress`) | Client components for the menu and the quantity stepper, React runtime |
| `rails` | Rails 8.1.4 | ssr | Ruby 3.4 with YJIT: ERB layouts and partials, Propshaft, tailwindcss-rails | Puma 8, one worker process per core, 3 threads each | N Puma workers (single mode for 1) | `Rack::Deflater` defaults | Hotwire (Turbo and Stimulus) through importmap |
| `aspnet` | ASP.NET Core 10.0.12 | ssr | C#: Razor Pages, Minimal API with source-generated `System.Text.Json` | Kestrel, .NET thread pool | `DOTNET_PROCESSOR_COUNT=N` | Response Compression middleware defaults (Brotli and gzip, `Fastest`) | Plain JS, about 1.9 KB |
| `go-templ` | Go 1.26.8 + templ 0.3.1020 | ssr | Go: `net/http` `ServeMux`, templ components, `encoding/json` | `net/http`, one goroutine per connection | `GOMAXPROCS(N)` | klauspost `gzhttp` defaults (gzip and zstd) | Plain JS, about 1.9 KB |
| `django` | Django 6.1.3 | ssr | Python 3.13: Django templates, WhiteNoise | Gunicorn 27 `gthread`, one worker process per core, one thread each | N worker processes | `GZipMiddleware` defaults | Plain JS, about 1.9 KB |
| `phoenix` | Phoenix 1.8.16 | ssr | Elixir 1.18 on OTP 28: controllers, HEEx function components, run as a `mix release` | Bandit 1.14, one BEAM scheduler per core | `+S N:N +SDcpu N:N` | Bandit's built-in compression (zstd, gzip, deflate) | esbuild-bundled plain JS, 1.2 KB minified |

Each app's `README.md` has the details and every deviation from the framework defaults.

## Adding an app

1. Create `apps/<id>/` with the app, its lockfile, a `README.md` noting framework-specific decisions, and an `app.toml` as described in [App manifest](SPEC.md#app-manifest-appsidapptoml).
2. Implement the routes and markup from [`SPEC.md`](SPEC.md) and run `cargo bench-web verify --only <id>` until it passes.
3. Run `cargo bench-web run --profile quick --only <id>,axum-maud` to check that the app builds, starts and serves every scenario without errors.
4. Fill in the app's row in the table above.
