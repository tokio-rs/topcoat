# Harbor Market: Axum + Maud (reference app)

The reference implementation of the Harbor Market storefront. Every other app ports its markup, copy and behavior; see the page anatomy in [`SPEC.md`](../../SPEC.md).

## Stack

| Piece | Version |
|-------|---------|
| [Axum](https://github.com/tokio-rs/axum) on Tokio, HTTP/1.1 | axum 0.8.9, tokio 1.54.1 |
| [Maud](https://maud.lambda.xyz) compile-time HTML templates | 0.27.0 |
| tower-http `CompressionLayer` (gzip, deflate, br, zstd) | 0.7.1 |
| axum-extra `CookieJar` for the `cart_count` cookie | 0.12.6 |
| `time` for typed dates and date formatting | 0.3.55 |
| serde / serde_json | 1.0.229 / 1.0.154 |
| Tailwind CSS standalone CLI from the benchmark shell, run by `build.rs` | 4.3.3 |
| Client-side behavior | `assets/app.js`, plain JS, about 1.9 KB |

## Running

Build inside the benchmark shell from the repository root, so the toolchain and Tailwind match every other app:

```sh
nix develop .#benchmarks
cd benchmarks/apps/axum-maud
cargo run --release                     # http://127.0.0.1:3000, data from ../../data
PORT=8080 BENCH_THREADS=4 BENCH_COMPRESSION=on cargo run --profile benchmark --features jemalloc
```

`build.rs` runs the `tailwindcss` on `PATH` and prints a build warning unless it reports version 4.3.3 (`TAILWIND_VERSION` in `build.rs`), the shell's version. The benchmark runner also checks tool versions against `expected_tools` in `bench.toml` and records them.

## Layout

- `src/data/`: the dataset types and the `Store`, which loads the JSON files and builds every index at startup (category lookup, per-category and global sort orders, featured list, orders newest first, order totals).
- `src/views/`: shared components, one per file (layout, header, footer, product card, product image, stars, pagination, breadcrumbs, status badge, color palette, icons).
- `src/pages/`: one module per HTML page, each with its handler.
- `src/api/`: the JSON handlers.
- `src/format.rs`, `src/urls.rs`, `src/cart.rs`: money, count and date formatting, link builders, and the cart cookie extractor.

## Judgment calls

- Components are plain functions returning `Markup`, the partials pattern from the Maud docs. Class strings are literal, and data-dependent ones come from `match` tables so Tailwind sees them.
- The stylesheet and script are embedded with `include_str!` and served under content-hashed URLs with a one year `immutable` cache header. Hashes are 64-bit FNV-1a, computed in `build.rs`.
- Dates are parsed into `time` types at startup, like typed database columns. Formatting happens per request with `time`'s own formatter: `Display` wrappers in `src/format.rs` call `.format()` with descriptions from `time::macros::format_description!`, such as `[month repr:short] [day padding:none], [year]`. Money and thousands separators are small hand-written `Display` impls, since Rust's standard library has no number formatting with grouping.
- The order summary (revenue, average, refunds) is computed at startup, like an aggregate query. Rows and money are still formatted per request.
- Description paragraphs are split on blank lines per request.
- Compression uses `CompressionLayer::new()` defaults: all four encodings, brotli at quality 4, nothing below 32 bytes.
- No tracing subscriber is installed, so nothing is logged per request. Startup errors go to stderr.
- Query and JSON extractor rejections are mapped to the 422 responses from `SPEC.md` instead of Axum's default 400 plain-text bodies.
- The cart handler extracts the body as a JSON object (`Json<serde_json::Map<..>>`) and then deserializes `CartRequest` from it. Deserializing the struct directly would also accept arrays such as `[1, 2]`, because serde builds structs from sequences; the SPEC only allows objects.
- No middleware is removed: Axum adds none by default. The only layer is `CompressionLayer`, when `BENCH_COMPRESSION=on`.
