# Benchmarks

Compare server rendering performance using equivalent storefront apps. The harness builds each app, starts its production server, and sends requests over loopback.

## What is measured

The load test measures the time to receive a full HTML document over HTTP/1.1 keep-alive connections. It does not run a browser or fetch page resources.

| Route | Content |
|---|---|
| `/` | Storefront home page |
| `/products?page=3&sort=price` | Sorted product listing |
| `/products/42` | Product detail |

The apps share the data in `data/products.json` and use matching filtering, sorting, pagination, and formatting rules. Run `scripts/verify_parity.sh` after changing an app. It compares the visible text of rendered pages. Check visual changes in a browser as well.

## Prerequisites

Run the scripts from the repository root. The harness records system metadata using macOS commands.

Install the tools required by the apps you want to measure:

- The repository's Rust toolchain and Topcoat CLI: `cargo install --path crates/topcoat-cli`.
- [oha](https://github.com/hatoo/oha), `jq`, and `curl`.
- Node.js and pnpm for the Next.js app.
- [cargo-leptos](https://github.com/leptos-rs/cargo-leptos) and the `wasm32-unknown-unknown` Rust target for the Leptos app.

Initial builds need network access to download dependencies and build tools. The app manifests and build scripts define their tool versions.

## Run the comparison

```sh
# Build and measure every app.
benchmarks/scripts/bench.sh

# Choose apps.
benchmarks/scripts/bench.sh topcoat leptos

# Run a shorter comparison while editing.
DURATION=5s WARMUP=2s RUNS=1 benchmarks/scripts/bench.sh

# Limit each Rust server to one Tokio worker thread.
SINGLE_THREAD=1 benchmarks/scripts/bench.sh

# Compare rendered content.
benchmarks/scripts/verify_parity.sh

# Rebuild a table from saved results.
benchmarks/scripts/compare.sh benchmarks/results/<timestamp>
```

Results, logs, and the summary are saved in `benchmarks/results/<timestamp>/`.

Each route is measured at an unrestricted request rate for throughput, and at a fixed rate for latency. Set `RATE` below the server's capacity if you want to reduce queueing in latency measurements. Use `DURATION`, `WARMUP`, `RUNS`, and `CONNECTIONS` to adjust the run. See [bench.sh](scripts/bench.sh) for defaults.

The summary takes the median across runs for each route, then averages across routes. Keep the raw results when a particular route matters to your comparison.

### Run an app manually

Build the Topcoat binary and its assets with the same profile:

```sh
cargo build --release -p storefront-topcoat
topcoat asset bundle --release --package storefront-topcoat
PORT=8090 ./target/release/storefront-topcoat
```

For the other apps, run from their own directories:

```sh
# benchmarks/nextjs
pnpm install
pnpm build
pnpm start

# benchmarks/leptos
cargo leptos build --release
LEPTOS_SITE_ADDR=127.0.0.1:8090 LEPTOS_SITE_ROOT=target/site ./target/release/storefront-leptos

# benchmarks/axum-maud
cargo build --release
PORT=8090 ./target/release/storefront-axum-maud
```

See [common.sh](scripts/common.sh) for the exact build and launch commands used by the harness.

## Interpreting results

- Compression is disabled to measure rendering and serving without compression work.
- Response sizes differ because frameworks may include scripts and serialized data. Compare the `bytes/resp` column alongside throughput and latency.
- The apps render each request on the server. The parity check verifies that Next.js responses disable caching.
- The Leptos app uses islands mode and waits for the complete document before responding.
- The Axum + Maud app provides a baseline using explicit handlers and templates.
- The Rust servers use multiple Tokio workers by default. Next.js runs in one Node process. `SINGLE_THREAD=1` limits Tokio workers; it does not pin processes to a CPU.
- The server and load generator share a machine. Close other heavy processes and repeat runs before drawing conclusions.

If high connection counts cause file descriptor errors, raise the limit with `ulimit -n 4096`.

## Files

App directories contain the implementations. Shared data and its generator live in `data/`, the harness lives in `scripts/`, and generated results go in the ignored `results/` directory.
