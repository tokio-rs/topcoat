# Harbor Market: Leptos

The Harbor Market storefront as a full-stack [Leptos](https://leptos.dev) app: server-side rendering with Axum and hydration of the whole page by a WebAssembly bundle. It follows the layout of the official [`start-axum`](https://github.com/leptos-rs/start-axum) template and is built with [`cargo-leptos`](https://github.com/leptos-rs/cargo-leptos). The sibling app [`leptos-islands`](../leptos-islands) is the same code in islands mode.

## Stack

| Piece | Version |
|-------|---------|
| Leptos, `leptos_router`, `leptos_meta` | 0.8.23, 0.8.17, 0.8.7 |
| `leptos_axum` on Axum and Tokio, HTTP/1.1 | 0.8.10, axum 0.8.9, tokio 1.54.1 |
| tower-http `CompressionLayer` | 0.7.1 |
| `cargo-leptos` | 0.3.10 (from the Nix shell) |
| `wasm-bindgen` (crate pinned to match the CLI), `wasm-opt` | 0.2.129, binaryen 132 |
| Tailwind CSS, run by cargo-leptos from `PATH` | 4.3.3 |
| `time`, serde, serde_json | 0.3.55, 1.0.229, 1.0.154 |

## Running

```sh
cargo leptos build --release            # server in target/benchmark, assets in target/site
LEPTOS_OUTPUT_NAME=harbor-leptos LEPTOS_SITE_ROOT=target/site LEPTOS_ENV=PROD \
  PORT=3000 target/benchmark/harbor-leptos
```

`cargo leptos build --release --features jemalloc` selects the allocator. The `LEPTOS_*` variables are the runtime configuration from the [deployment guide](https://book.leptos.dev/deployment/ssr.html); `app.toml` sets them for the runner.

## Layout

- `src/main.rs`, `src/config.rs`: the server binary. One Axum router with the Leptos routes, the JSON API and the static file fallback.
- `src/app.rs`: the HTML shell and the `App` component with the router and the shared layout.
- `src/pages/`: one module per page. Each data page has a `#[server]` function that loads its data and a component that renders it.
- `src/components/`: shared components, one per file, ported one-to-one from the reference.
- `src/model/`: data types shared by the server and the browser. `src/store/` (server only) loads the dataset and builds the indexes at startup.
- `src/api/` (server only): the JSON handlers.

## Build and settings

- **Profiles.** cargo-leptos builds the server with the shared `benchmark` profile (`bin-profile-release = "benchmark"`, so `target/{profile}` in `app.toml` matches) and the WebAssembly bundle with the size-optimized `wasm-release` profile from the Leptos book's [binary size guide](https://book.leptos.dev/deployment/binary_size.html) (`opt-level = "z"`, `lto`, one codegen unit, `panic = "abort"`, as in the template). In release mode cargo-leptos also runs `wasm-opt -Oz` and minifies the JS glue. The nightly-only `build-std` step from the guide is not used.
- **Features.** `{features}` goes to `cargo leptos build`, which passes it to both the server and the WebAssembly build. The allocator crates are target-specific dependencies, so `--features jemalloc` only affects the server.
- **Threads.** The Tokio runtime is built by hand like in the reference: `BENCH_THREADS=N` sets `worker_threads(N)`, unset or `0` keeps Tokio's default of one worker per core. Leptos renders on the request's worker thread.
- **Compression.** `BENCH_COMPRESSION=on` wraps the router in `CompressionLayer::new()` with its defaults, the same layer as the reference. (`leptos_axum` itself depends on tower-http 0.6 for `ServeDir`; the app uses 0.7.1 to compress exactly like the reference.)
- **Logging.** No tracing subscriber or logger is installed, so nothing is logged per request.
- **Template pieces left out.** The `start-axum` template adds no middleware layers, so none were removed. Its shell includes `<AutoReload>`, the `cargo leptos watch` live-reload script, which renders nothing unless `LEPTOS_WATCH` is set; it is left out. The server binds `127.0.0.1:$PORT` itself instead of using `site-addr` from the Leptos configuration. The store is loaded before the listener is bound, so `/health` only answers once all data is in memory.
- **Tailwind.** `tailwind-input-file` in `[package.metadata.leptos]`; cargo-leptos runs the `tailwindcss` from `PATH` (4.3.3 in the shell). It then post-processes the stylesheet with lightningcss for its default browserslist query (`defaults`), which adds fallbacks for older browsers. The stylesheet is therefore about 41 KB instead of the reference's 33 KB with the same classes; that is cargo-leptos's standard pipeline and is left as is.

## Judgment calls

- **Data loading.** The store is loaded once at startup and given to Leptos through `leptos_routes_with_context` (and the fallback handler), so components and server functions read it with `expect_context::<Arc<Store>>()`; the Axum JSON handlers get the same `Arc<Store>` as Axum state. Each data page loads its data the way the Leptos book describes for SSR apps: a `#[server]` function wrapped in a `Resource` and rendered under `<Suspense>`. During SSR the server function runs in process, and the resolved value is serialized into the page so the browser can hydrate without a second request. Client-side navigations call the same function over HTTP.
- **Only what the page renders is serialized.** The server functions return page-shaped data: product cards carry a `ProductSummary` (name, tagline, category name, price, rating, stock, color) instead of whole products with descriptions and reviews, and shared lists (`categories`, `orders`) are `Arc` slices from the store, so nothing is cloned to build them. What remains is the cost of whole-app hydration; see the sizes below.
- **SSR mode.** The data pages use `SsrMode::Async`; `/about` and the 404 fallback keep the default out-of-order streaming. The default mode streams `<Suspense>` content after the shell in `<template>` chunks that JavaScript moves into place, and has, in the words of the [`SsrMode` docs](https://docs.rs/leptos_router/0.8.17/leptos_router/enum.SsrMode.html), "weaker meta tag support". Here every data page's `<title>` and HTTP status (404 for an unknown product, 422 for a bad filter) depend on the data, and a storefront's content must be in the HTML for crawlers and users without JavaScript. `Async` waits for the page's resources and renders in one pass, which the docs recommend for meta tags; with the data in memory it costs no time to first byte. Out-of-order streaming with `Resource::new_blocking` got the title and status right but still delivered the content as out-of-order chunks, and `PartiallyBlocked` fixes that with extra string manipulation that the docs call slower.
- **Error pages.** The server functions return a custom error type, `PageError`, implementing `FromServerFnError` (the 0.8 way to return typed errors). The error component sets the status with `ResponseOptions`. Unknown paths, including under `/api`, reach `file_and_error_handler`, which serves files from `target/site` or renders the app, whose `<Routes fallback>` is the 404 page.
- **JSON API.** `/health`, `/api/products`, `/api/products/{id}` and `/api/cart` are plain Axum handlers on the same router, as in the Leptos Axum examples. Server functions are RPC endpoints (POST to generated URLs with their own encoding), not a public REST API, so they are only used for page data.
- **Cart count.** The `App` reads the `cart_count` cookie through a `SharedValue`: on the server it uses axum-extra's `CookieJar` on the request headers (as the reference does), and the value is serialized so hydration starts from the same number. The add-to-cart button reads the cookie again in the browser, where `document.cookie` is a single `name=value; ...` string, so `cart.rs` splits that string itself. It lives in an `RwSignal<CartCount>` in context; the header badge reads it and the add-to-cart button sets it after writing the cookie with `web-sys`.
- **Formatting.** Dates use the `time` crate's `format_description!` formatter on values parsed at load, as in the reference; money and counts use small hand-written functions because Rust's standard library has no locale-aware number formatting. `format.rs` is shared with the WebAssembly client, which formats the same values during hydration and client-side navigation.
- **Links.** Leptos has no URL generation from route definitions: the router docs and examples write `href` strings on `<a>` or `<A>`, and the router handles clicks on plain `<a>` elements too. Links are plain `<a href>` built with the same small `ListingUrl` and `ProductUrl` builders as the reference (`leptos_router` has a `ParamsMap::to_query_string`, but it serializes a parsed query map and is not documented for building links). `<A>` is not used because it adds `aria-current` to every link matching the current URL, which would change the reference markup, and it only adds relative-route resolution that absolute paths do not need.
- **Interactivity.** The menu toggle and the quantity stepper are ordinary components with signals. The nav's class switches between two literal class strings so the markup and order match the reference and Tailwind sees both.
- **Client-side routing.** After hydration `leptos_router` handles same-origin link clicks. The footer's link to `/api/products` has `rel="external"`, the documented way to let the browser load a URL the router does not own.
- **Static assets.** The JS, WASM and CSS are served by `leptos_axum`'s file handler (tower-http `ServeDir`) under fixed names in `/pkg`, revalidated with `Last-Modified`. cargo-leptos can hash file names (`hash-files`), but in Leptos 0.8.23 `HydrationScripts` and `HashedStylesheet` then read `hash.txt` from disk on every render, so it is left off as in the template.
- **Per-request extras the framework adds.** `leptos_axum` generates a CSP nonce for every response and puts it on the hydration script tags.

## What hydration costs

HTML per page with `cart_count=3`, raw / gzip -9 bytes, against the reference:

| Page | Reference | Leptos | Serialized resource data |
|------|-----------|--------|--------------------------|
| `/about` | 9,019 / 2,692 | 10,099 / 3,157 | 149 (cart count only) |
| `/` | 30,491 / 4,653 | 35,829 / 5,810 | 4,331 |
| `/products?category=outdoor&sort=price-asc&page=1` | 60,638 / 5,331 | 70,675 / 7,664 | 8,863 |
| `/products?page=3` | 60,422 / 5,644 | 70,240 / 8,286 | 8,670 |
| `/products/61` | 30,510 / 4,906 | 35,472 / 6,082 | 4,012 |
| `/admin/orders` | 338,581 / 22,394 | 455,947 / 38,855 | 116,209 |

Almost all of the difference is the serialized resource data (JSON inside a JS string, so quotes are escaped) plus about 1 KB of hydration script and preload links; comment markers add only tens to a few hundred bytes. The orders page ships every order a second time as data.

Client assets: `harbor-leptos.wasm` 862 KB (322 KB gzip, 248 KB brotli), `harbor-leptos.js` 24 KB (6.9 KB gzip), stylesheet 41 KB. The runner finds the WASM through the `<link rel="preload">` that `HydrationScripts` emits, so `extra_assets` stays empty. It also reports `harbor-leptos_bg.js` and `harbor-leptos_bg.wasm` as missing: those names are string literals in wasm-bindgen's glue (its default URL when `init` is called without a path), which the browser never requests because the hydration script passes the real path.
