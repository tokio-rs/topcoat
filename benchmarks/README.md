# Benchmarks

Server-rendering performance comparison between **Topcoat**, **Next.js**,
**Leptos**, and a hand-written **Axum + Maud** app on a realistic storefront
app, plus the harness to run it.

## What is measured

HTTP document requests against each framework's production server on loopback,
over HTTP/1.1 keep-alive connections. Three routes are exercised:

| Route | What it renders |
|-------|-----------------|
| `/` | shared layout, hero section, 12 featured product cards |
| `/products?page=3&sort=price` | 24-card grid filtered/sorted via query params, filter chips, pagination |
| `/products/42` | breadcrumbs, product detail, spec table, reviews, related products |

This is a server-rendering benchmark, not a browser benchmark: it measures
time-to-full-HTML-document, not time-to-interactive, and no subresources
(CSS, JS, images) are fetched during the load test.

## The demo app

The same storefront is implemented four times, in `topcoat/`, `nextjs/`,
`leptos/`, and `axum-maud/`. All four:

- render the identical component tree (nav, footer, product cards, rating
  stars, pagination, spec table, review list, breadcrumbs) with identical
  Tailwind v4 utility classes,
- load the identical checked-in data set (`data/products.json`, 500 seeded
  products; regenerate with `node data/generate.mjs > data/products.json`),
- implement identical query semantics (page clamping, the four sort orders
  with id tie-breaks, category filtering) and identical formatting rules
  (integer-math prices and ratings, verbatim ISO dates).

`scripts/verify_parity.sh` enforces this: it renders five routes in every
framework, reduces the HTML to visible text, and diffs the results. Run it
after touching any of the apps.

## Prerequisites

- Rust (the repo toolchain) and the `topcoat` CLI (`cargo install --path
  crates/topcoat-cli`)
- [oha](https://github.com/hatoo/oha): `brew install oha`
- Node.js >= 20 and pnpm (Next.js app)
- [cargo-leptos](https://github.com/leptos-rs/cargo-leptos):
  `cargo install cargo-leptos --locked`, plus
  `rustup target add wasm32-unknown-unknown`
- `jq` and `curl`

The first Topcoat, Leptos, and Axum + Maud builds download the standalone
Tailwind CLI and need network access. Topcoat and Axum + Maud pin 4.3.2;
cargo-leptos prefers a `tailwindcss` found on `PATH` (e.g. from Homebrew) and
only honors `LEPTOS_TAILWIND_VERSION=v4.3.2` when none is installed. Any Tailwind v4
produces the same rules for the utility classes this app uses;
`verify_parity.sh` plus a visual spot-check cover the difference.

## Running

```sh
# Everything: builds each app, runs the full matrix, prints the table.
benchmarks/scripts/bench.sh

# One or two frameworks only:
benchmarks/scripts/bench.sh topcoat
benchmarks/scripts/bench.sh topcoat leptos

# Faster, noisier run while iterating:
DURATION=5s WARMUP=2s RUNS=1 benchmarks/scripts/bench.sh

# One core each: the Rust servers run single-threaded, like next start.
SINGLE_THREAD=1 benchmarks/scripts/bench.sh

# Re-render the table for any saved results directory:
benchmarks/scripts/compare.sh benchmarks/results/<timestamp>

# Cross-framework rendered-content check:
benchmarks/scripts/verify_parity.sh
```

Raw oha JSON, server logs, and `summary.md` land in
`benchmarks/results/<timestamp>/`.

Each framework is measured in two modes per route:

- **throughput**: fixed connection count, unbounded rate; the `req/s` column.
- **fixed rate** (`RATE`, default 200 req/s): below saturation for every
  framework so the latency percentile columns mostly measure service time.
  Note that 200 req/s is already a substantial fraction of Next.js's
  capacity on the heaviest route, so its tail latencies include some
  queueing; lower `RATE` for a pure service-time comparison.

Defaults: 3 runs x 20s per route and mode, 32 connections, medians reported.

### Running one app manually

Prefix either Rust server with `TOKIO_WORKER_THREADS=1` to run it single-threaded
(what `SINGLE_THREAD=1` does under the hood).

```sh
# Topcoat (release binary + dev-mode asset bundle):
cargo build --release -p storefront-topcoat
topcoat asset bundle --package storefront-topcoat
PORT=8090 ./target/release/storefront-topcoat

# Next.js:
cd benchmarks/nextjs && pnpm install && pnpm build && pnpm start

# Leptos:
cd benchmarks/leptos && LEPTOS_TAILWIND_VERSION=v4.3.2 cargo leptos build --release
cd benchmarks/leptos && LEPTOS_SITE_ADDR=127.0.0.1:8090 LEPTOS_SITE_ROOT=target/site \
    ./target/release/storefront-leptos

# Axum + Maud:
cd benchmarks/axum-maud && cargo build --release
cd benchmarks/axum-maud && PORT=8090 ./target/release/storefront-axum-maud
```

## Fairness notes

- **Compression is disabled in every server.** The benchmark measures raw
  framework rendering and serving performance, not the throughput of a
  compression codec: these servers render a page faster than any codec
  compresses it, so with compression on the comparison mostly measures the
  codec. Topcoat's default response compression is switched off
  (`Compression::off()`), Next.js sets `compress: false` in `next.config.ts`
  (`next start` gzips by default), and Leptos and Axum + Maud add no
  compression middleware. Real deployments should leave compression on; the
  per-response CPU cost buys a many-times-smaller transfer.
- **Response sizes intentionally differ.** Topcoat and Axum + Maud ship plain
  HTML with zero JavaScript. Next.js embeds its RSC payload and script tags;
  Leptos embeds hydration data and the wasm loader. That is each framework's realistic
  production output for this app, and the `bytes/resp` column keeps the
  difference visible. Larger documents cost real time to render and transfer,
  so this is part of the comparison, not noise in it.
- **Next.js truly server-renders every request.** All pages set
  `export const dynamic = "force-dynamic"`; the build output lists them as
  dynamic and responses carry `Cache-Control: ... no-store` (asserted by
  `verify_parity.sh`).
- **Leptos uses `SsrMode::Async`**, so each response is one complete document
  (comparable to the others), not an out-of-order stream.
- **Axum + Maud is the hand-written baseline.** It renders the same component
  tree as plain functions returning `maud` compile-time templates, with axum
  doing the routing and query parsing; there is no framework layer on top.
  Its stylesheet is generated at build time by the same pinned standalone
  Tailwind CLI the Topcoat app uses (a build-script dependency only; nothing
  from Topcoat is linked into the server binary).
- **Stock release profiles.** Both Rust apps build with an untuned
  `cargo --release` (no LTO or codegen tweaks); Next.js uses a plain
  `next build`.
- **Process models differ.** By default the Rust servers use every core, while
  `next start` is a single Node process, so JS rendering is effectively
  single-threaded. Real Node deployments scale by running multiple instances;
  read the default Next.js rows as per-instance numbers. For an apples-to-apples
  one-core comparison, run with `SINGLE_THREAD=1`: it pins the Rust servers to
  a single Tokio worker thread (`TOKIO_WORKER_THREADS=1`), matching the one
  Node process, and the rendered table is labelled `single-threaded`. (This
  caps the async runtime's worker threads; the OS still schedules that thread
  across cores rather than hard-pinning it.)
- **Same machine, one server at a time.** The load generator shares the
  machine with the server, so absolute numbers are pessimistic and only the
  relative comparison is meaningful. Close other heavy processes before a
  measured run.
- If oha reports connection errors at high connection counts, raise the file
  descriptor limit (`ulimit -n 4096`).

## Layout

```
data/       seeded product data set + generator (single source of truth)
topcoat/    Topcoat app (workspace member `storefront-topcoat`)
nextjs/     Next.js 15 App Router app
leptos/     Leptos 0.8 + Axum app (own cargo workspace, excluded from the root)
axum-maud/  Axum 0.8 + Maud app (own cargo workspace, excluded from the root)
scripts/    bench.sh, compare.sh, verify_parity.sh, helpers
results/    benchmark output (gitignored)
```
