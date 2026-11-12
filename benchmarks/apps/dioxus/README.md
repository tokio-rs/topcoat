# Harbor Market: Dioxus

The Harbor Market storefront as a [Dioxus](https://dioxuslabs.com) 0.7 fullstack app. Every page is rendered on the server with Axum and then hydrated in the browser by the same `rsx!` components compiled to WebAssembly. See the page anatomy in [`SPEC.md`](../../SPEC.md).

## Stack

| Piece | Version |
|-------|---------|
| Dioxus fullstack (`dioxus` with `fullstack` and `router`), server-side rendering plus hydration | 0.7.10 |
| `dx` CLI, which builds both halves with `dx bundle` | 0.7.10 |
| Axum on Tokio (the server Dioxus runs on), HTTP/1.1 | axum 0.8.9, tokio 1.54.1 |
| tower-http `CompressionLayer` (gzip, deflate, br, zstd) | 0.6.11 |
| axum-extra `CookieJar` for the `cart_count` cookie | 0.10.3 |
| `time` for dates, parsed at load and formatted with `format_description!` | 0.3.55 |
| serde / serde_json | 1.0.229 / 1.0.154 |
| wasm-bindgen / web-sys (client) | 0.2.129 / 0.3.106 |
| Tailwind CSS standalone CLI, run by `dx` | 4.3.3 |
| `wasm-opt` (binaryen) and `esbuild`, run by `dx` | 132 / 0.27.2 |

`Cargo.lock` pins everything. `wasm-bindgen` is pinned to `=0.2.129` because `dx` requires the crate and the `wasm-bindgen` CLI to match exactly, and the Nix shell provides 0.2.129.

## Building and running

```sh
# From the repository root, inside `nix develop .#benchmarks`.
cd benchmarks/apps/dioxus
NO_DOWNLOADS=1 dx bundle --release --out-dir dist \
  @client --web --profile wasm-release --debug-symbols false \
  @server --server --profile benchmark --features jemalloc
PORT=8080 BENCH_DATA_DIR=$PWD/../../data dist/server
```

`dx bundle` is the documented production path for fullstack apps. It runs Tailwind on `tailwind.css`, builds the WebAssembly client, runs `wasm-bindgen`, `wasm-opt` and esbuild, hashes the assets, builds the server and writes `dist/server` next to `dist/public/`. The server serves `public/` itself.

`NO_DOWNLOADS=1` makes `dx` use the `tailwindcss`, `wasm-bindgen` and `wasm-opt` binaries on `PATH` rather than downloading its own. Without it, `dx` 0.7.10 would download Tailwind 4.1.5 instead of using the shell's 4.3.3. The nixpkgs `dx` is built with the `no-downloads` feature anyway.

### Build profiles

- **Server:** `@server --profile benchmark`, the same `[profile.benchmark]` as every other Rust app (`inherits = "release"`, `codegen-units = 1`, `lto = "fat"`). `dx` lets each side pick a profile, so the release profile is unchanged. The `{features}` placeholder in `app.toml` only reaches the server, because the allocator features only make sense there.
- **Client:** `@client --profile wasm-release` (the profile name `dx` uses for release web builds by default), with the size settings from the [Dioxus optimization guide](https://dioxuslabs.com/learn/0.7/guides/tips/optimizing): `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`, `debug = false`. `wasm-opt` runs at level `z`, which is the `dx` default and is spelled out in `Dioxus.toml`.
- **`--debug-symbols false`:** `dx bundle` keeps DWARF sections in the `.wasm` file by default (about 350 KB in this app). A production deploy strips them, so the client build turns them off.

Release builds of Dioxus have no dev server, hot reloading or devtools websocket: `dioxus::serve` falls through to a plain `axum::serve` when `debug_assertions` is off.

## Serve configuration

The server uses `dioxus::server::router(App)`, which is `ServeConfig::new()` with its defaults:

- **Streaming disabled** (`StreamingMode::Disabled`, the default). The server waits for every `use_server_future` to resolve and sends the whole page in one response, so the status code is known before the first byte.
- **No incremental rendering (ISR) cache.** `ServeConfig::incremental` is never called, so `SsrRendererPool` has no cache and every request runs the components.
- **No static site generation.** `dx bundle` runs without `--ssg`.

The `age: 0` header on HTML responses is the freshness header Dioxus writes for every fresh render; nothing is cached.

## How the app is built

- **Routing:** a `#[derive(Routable)]` enum in `src/routes.rs` with a `#[layout(Shell)]` for the shared layout. The product id is a typed `u32` segment, so `/products/abc` falls through to the `/:..segments` route that renders the 404 page. The listing takes its whole query string as a `ListingQuery` (`/products?:..query`), the router's documented hook for custom query types: it keeps the raw values so the server can answer `sort=bogus` with 422, and its `Display` writes the canonical form (parameters in the order `category`, `sort`, `page`, defaults left out).
- **Data loading:** each page calls a `#[server]` function through `use_server_future`, as the [SSR guide](https://dioxuslabs.com/learn/0.7/essentials/fullstack/ssr) recommends for data that must be in the server-rendered HTML. During SSR the server function runs in-process; its result is serialized into the page so the client hydrates without calling the server again. The page modules in `src/pages/` hold their server function and the shape of the data they send.
- **Server-only data:** the dataset is loaded once at startup into a `Store` with every index built up front (`src/store.rs`), and server functions read it through a `static OnceLock`. This is the "global static" option from the [Router and State guide](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum); it is initialized eagerly in `main` so loading is never part of a request.
- **Cart count:** the `Shell` layout loads it with a `cart_count` server function that takes the request's `HeaderMap` as a server-only extractor and reads the cookie with axum-extra. The value is provided as a `Signal<CartCount>` context, which the header badge reads and the add-to-cart button writes.
- **Status codes:** server functions fail with `HttpError` 404 or 422, which reach the component as `ServerFnError::ServerError { code, .. }`. The page then renders `ErrorPage`, which calls `FullstackContext::commit_http_status` so the response carries the status. The error is part of the serialized result, so the browser hydrates the same error page. (Throwing to an `ErrorBoundary` sets the status too, but the boundary's hydration data only keeps the error message, so the client could not tell 404 from 422.)
- **JSON API and `/health`:** plain Axum handlers added to the router next to the Dioxus app (`src/api/`), as in the [Router and State guide](https://dioxuslabs.com/learn/0.7/essentials/fullstack/axum). Server functions have their own wire format (POST with a JSON body, errors as `ServerFnError` objects), while the public API needs fixed URLs, query strings, exact JSON shapes and `{"error": ...}` bodies. A Dioxus team publishing a REST API would write it this way and keep server functions for the app itself. The handlers read the same `Store` as the server functions.
- **Dates** are `time::Date` and `time::OffsetDateTime`, parsed while the dataset loads through serde (`time::serde::format_description!` for dates, `time::serde::rfc3339` for order timestamps). Pages format them per request with `time`'s `format_description!` (`[month repr:short] [day padding:none], [year]` and so on) in `src/format.rs`. The same code runs in the WebAssembly client during hydration; `time` compiles to wasm32 without extra features. Money and thousands separators stay small hand-written `Display` types, as Rust's standard library has no formatter for them.
- **Interactivity:** the menu toggle (`views/header.rs`) and the quantity stepper with add-to-cart (`pages/detail.rs`) are signals and `onclick` handlers. The cookie is read and written with `web-sys`.
- **Links:** every internal URL comes from a `Route` value (`Route::Detail { id }`, `Route::category(slug)`, `Route::listing(category, sort, page)`), and links use the router's `Link` component as in the Dioxus docs. The first page load is server-rendered and hydrated; after that, `Link` navigates on the client and the next page's `use_server_future` calls its server function over HTTP, like `next/link` in the Next.js app. A few links stay plain `a` elements:
  - The listing's category sidebar and sort links, with their `href` still built from `Route`. The SPEC marks the current sort with `aria-current="true"` and the current category on every page, but `Link` always adds its own `aria-current="page"` to a link whose URL equals the current one, and passing a second value renders the attribute twice.
  - The anchors `/about#shipping`, `/about#warranty`, `/about#contact` and `#reviews`: `Route::About` has no hash fragment, and an anchor should scroll rather than route.
  - `/api/products` in the footer, which is a JSON endpoint and not a page of the router (as a `Link` it would client-route to the 404 page).
- **Tailwind** is integrated the Dioxus 0.7 way: `dx` notices `tailwind.css` in the crate root, compiles it to `assets/tailwind.css`, and the `App` component links it with `document::Stylesheet { href: asset!("/assets/tailwind.css") }`, which gives it a content-hashed URL with an immutable cache header. `tailwind.css` scans `src/` and `index.html`.
- **Shell HTML:** `index.html` sets `lang`, the meta tags, the icon link and the body classes. `dx` adds the hashed script (`type="module"`, plus a `preload` link), and the server adds the title, the stylesheet, a small inline streaming bootstrap script and the hydration data.

## Deviations from the reference markup

- **Root element:** Dioxus mounts the app into `<div id="main">`; the id is fixed in 0.7.10 (`ServeConfig::new` hard-codes it and the `IndexHtml` type that would change it is private). The div has the `contents` class so the body's flex layout still applies to the header, `main` and footer.
- **Skip link target:** because the root div already uses `id="main"`, `main` has `id="content"` and the skip link points to `#content`. Keeping `main id="main"` would duplicate the id and make the skip link jump to the root div.
- **`data-*` hooks** (`data-cart-count`, `data-product-form`, `data-step` and so on) are left out. They only exist for the reference's plain JS; the Dioxus components wire the same elements through signals.
- Dioxus adds `data-node-hydration` attributes and `<!--node-id-->` comments for hydration.
- **`aria-current` from `Link`:** `Link` marks every link to the current URL with `aria-current="page"`, so on `/products` the header and footer "All products" links carry it too, and on `/` the two logo links do. The pagination's current page gets the attribute this way, as the SPEC asks.
- **`/products?`:** the route for the unfiltered first page displays as `/products?`. The router writes the `?` of a `?:..query` route even when the query is empty. It loads the same page as `/products`.

## Threads and compression

- `BENCH_THREADS`: unset or `0` keeps Tokio's default of one worker per core; `N` sets `worker_threads(N)` on a runtime built in `main`, and `dioxus::serve` runs on it.
- **Caveat:** Dioxus renders pages on its own `LocalPoolHandle` (a pool of single-threaded runtimes, because a `VirtualDom` is not `Send`), sized by `std::thread::available_parallelism()` and not configurable in 0.7.10. With `BENCH_THREADS=1` the server therefore has one Tokio worker plus one render thread per core, and page rendering still uses every core: in a quick run the HTML scenarios used about 6 cores with `--threads 1`, while plaintext and the JSON API stayed at 1. `available_parallelism` follows CPU affinity and cgroup limits on Linux, so running the server under `taskset` limits both pools there; macOS has no equivalent.
- `BENCH_COMPRESSION=on` adds `tower_http::compression::CompressionLayer::new()` (default settings) around the whole router, pages, API and assets alike. Dioxus has no compression of its own for rendered pages; tower-http's layer is the middleware its [docs](https://dioxuslabs.com/learn/0.7/essentials/fullstack/middleware) point to.
- Default middleware (rule 12): nothing is removed. `dioxus::server::router` sets up static assets, server functions and the SSR fallback, and the app adds its routes around it.
- `dioxus::serve` installs a tracing subscriber at `INFO` in release builds. `main` sets it to `WARN` first, so nothing is logged per request and errors still are.

## Payload

The HTML is larger than the reference because of hydration: the markers above (each `Link` also carries a click listener marker) plus `window.initial_dioxus_hydration_data`, which holds the server function results as base64-encoded CBOR. Sizes with the checked-in data (uncompressed, `cart_count=3`):

| Page | Reference HTML | Dioxus HTML | Hydration data |
|------|----------------|-------------|----------------|
| `/` | 30.5 KB | 50.4 KB | 8.7 KB |
| `/about` | 9.0 KB | 12.0 KB | 0.1 KB |
| `/products?category=outdoor&sort=price-asc&page=1` | 60.6 KB | 101.2 KB | 18.6 KB |
| `/products/1` | 34.3 KB | 56.7 KB | 10.2 KB |
| `/admin/orders` | 338.6 KB | 707.2 KB | 221.6 KB |

The orders page sends all 500 orders twice: once as HTML and once as hydration data, because the client re-runs the same component during hydration. That is how Dioxus fullstack works by default; nothing was trimmed to make it smaller.

Client assets, the same on every page (raw / brotli as measured by the runner):

| Asset | Raw | Brotli |
|-------|-----|--------|
| WebAssembly module | 1,125,192 B | 360,778 B |
| JS (wasm-bindgen glue and the Dioxus interpreter, minified by esbuild) | 56,041 B | 13,015 B |
| CSS (Tailwind, minified) | 41,069 B | 6,664 B |

The runner finds the hashed `.wasm` through the JS, so `extra_assets` stays empty. It also reports a few "missing" references (`/assets/harbor-dioxus_bg.wasm`, `/assets/snippets/...`): those are default paths and pre-bundling import paths that appear as string literals in the generated JS but are never requested.
