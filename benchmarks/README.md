# Benchmarks

Compare server rendering with equivalent storefront apps in Topcoat, Next.js, Leptos, and Axum + Maud.

## What is measured

The harness sends HTTP document requests to each production server over local HTTP/1.1 keep-alive connections. It measures complete HTML responses. It does not run a browser or fetch stylesheets, scripts, or images.

| Route | Content |
|-------|---------|
| `/` | Home page with featured products |
| `/products?page=3&sort=price` | Filtered and sorted product list with pagination |
| `/products/42` | Product details, reviews, and related products |

The apps use the same checked-in product data, page content, query behavior, and formatting rules. Run `benchmarks/scripts/verify_parity.sh` after changing an app to compare rendered text across implementations. This check does not replace a visual review of layout changes.

## Prerequisites

- The repository's Rust toolchain and the Topcoat CLI, installed with `cargo install --path crates/topcoat-cli`.
- [oha](https://github.com/hatoo/oha) for load generation.
- Node.js and pnpm for the Next.js app.
- [cargo-leptos](https://github.com/leptos-rs/cargo-leptos) and the `wasm32-unknown-unknown` Rust target for Leptos.
- `jq` and `curl`.

Initial Rust builds may need network access to download the standalone Tailwind CLI. Leptos uses `tailwindcss` from `PATH` when available. Otherwise, its build uses the version selected by `LEPTOS_TAILWIND_VERSION`.

## Running the benchmarks

Run commands from the repository root:

```sh
# Build and measure all apps.
benchmarks/scripts/bench.sh

# Measure selected apps.
benchmarks/scripts/bench.sh topcoat leptos

# Use shorter runs during development.
DURATION=5s WARMUP=2s RUNS=1 benchmarks/scripts/bench.sh

# Limit each Rust server to one Tokio worker thread.
SINGLE_THREAD=1 benchmarks/scripts/bench.sh

# Render a comparison from saved results.
benchmarks/scripts/compare.sh benchmarks/results/<timestamp>

# Check that the apps render equivalent content.
benchmarks/scripts/verify_parity.sh
```

Results are written to `benchmarks/results/<timestamp>/`, including raw oha output, server logs, and `summary.md`.

Each route is measured at an unrestricted request rate for throughput, then at a fixed rate for latency. Set `RATE` below the slowest server's capacity when comparing service latency. A higher rate introduces queueing, which can dominate the tail percentiles.

The summary takes the median of repeated runs for each route, then averages across routes. The harness script defines the default duration, run count, and connection count.

### Running an app manually

For Topcoat, build the release binary and bundle its assets:

```sh
cargo build --release -p storefront-topcoat
topcoat asset bundle --release --package storefront-topcoat
PORT=8090 ./target/release/storefront-topcoat
```

For Next.js, run these commands in `benchmarks/nextjs`:

```sh
pnpm install
pnpm build
pnpm start
```

For Leptos, run these commands in `benchmarks/leptos`:

```sh
LEPTOS_TAILWIND_VERSION=v4.3.2 cargo leptos build --release
LEPTOS_SITE_ADDR=127.0.0.1:8090 LEPTOS_SITE_ROOT=target/site ./target/release/storefront-leptos
```

For Axum + Maud, run these commands in `benchmarks/axum-maud`:

```sh
cargo build --release
PORT=8090 ./target/release/storefront-axum-maud
```

Set `TOKIO_WORKER_THREADS=1` when starting a Rust server to limit it to one runtime worker thread.

## Interpreting results

- **Compression is disabled.** The measurements focus on rendering and serving HTML. They do not include the CPU cost or transfer savings of response compression.
- **Response sizes differ.** Each app produces the output required by its framework. The `bytes/resp` column makes this difference visible.
- **Requests render fresh content.** Next.js pages force dynamic rendering. Leptos renders complete responses in islands mode without interactive islands. The Axum + Maud app provides a baseline built from handlers and templates.
- **Builds use standard release settings.** Check the app manifests before comparing results from different revisions.
- **Concurrency differs.** Rust servers use multiple runtime workers by default, while Next.js runs in one Node process. Use `SINGLE_THREAD=1` to compare with one Tokio worker per Rust server. This limits worker count but does not pin a process to a CPU core.
- **The load generator shares the server's machine.** Run one server at a time and close other heavy processes. Treat results as local comparisons rather than production capacity estimates.

If oha reports connection errors at high connection counts, check the file descriptor limit. For example, `ulimit -n 4096` raises it for the current shell.

## Files

The framework directories contain the app implementations. Shared input data lives in `data/`, the harness in `scripts/`, and generated output in the ignored `results/` directory. Regenerate the product data with:

```sh
node benchmarks/data/generate.mjs > benchmarks/data/products.json
```
