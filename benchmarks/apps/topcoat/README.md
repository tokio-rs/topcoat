# Harbor Market: Topcoat

The Harbor Market storefront from [`SPEC.md`](../../SPEC.md), written with Topcoat the way its guides teach it. The markup, copy and class strings are ported one-to-one from the reference app in [`axum-maud`](../axum-maud).

## Stack

| Piece | Version |
|-------|---------|
| [Topcoat](../../../README.md) from this repository (`topcoat` facade, default features) | 0.12.0 (path dependency) |
| Tokio multi-threaded runtime, hyper (through Topcoat's server) | 1.51.1, 1.9.0 |
| Tailwind CSS standalone CLI from the benchmark shell, run by `topcoat::tailwind::BuildConfig` | 4.3.3 |
| `time` for typed dates and their formatting (`format_description!`) | 0.3.49 |
| serde / serde_json | 1.0.228 / 1.0.149 |
| Client-side behavior | Topcoat's browser runtime (`topcoat::runtime::script()`), about 48 KB minified |

The app is a member of the repository's Cargo workspace, so it builds against the in-repo crates and shares the workspace's `Cargo.lock` and `target/` directory.

## Running

From the repository root, inside `nix develop .#benchmarks`:

```sh
cargo topcoat asset bundle --package harbor-topcoat   # builds the app and bundles its assets
PORT=3000 target/debug/harbor-topcoat
```

For the benchmark build, `app.toml` runs `cargo topcoat asset bundle --profile benchmark` and then `cargo build --profile benchmark` with the allocator feature.

## Settings

- **Threads.** `BENCH_THREADS=N` builds the Tokio multi-threaded runtime with `worker_threads(N)`. Unset or `0` keeps Tokio's default of one worker per core. `topcoat::start` serves the router on that runtime and binds `127.0.0.1:$PORT`.
- **Compression.** Topcoat compresses responses by default ([`Compression`](../../../crates/topcoat-router/src/compression.rs): gzip and brotli at the `Balanced` level, quality 4, skipping bodies under 32 bytes). `BENCH_COMPRESSION=on` keeps that default; `off` adds `Compression::disable()` to the router, as documented for apps behind a compressing proxy.
- **Logging.** Topcoat logs nothing per request, and no subscriber is installed.
- **Allocator.** The `jemalloc` and `mimalloc` features set the global allocator, like the other Rust apps.

## Layout

- `src/app.rs` and `src/app/`: the routes, with [module routing](../../../docs/router/module.md). `app.rs` holds the router, the root layout and the home page. `products.rs` is the listing, `products/product_id.rs` the detail page, `admin/orders.rs` the orders page, `api/` the JSON routes and `not_found.rs` the catch-all 404 page.
- `src/components/`: shared `#[component]`s, one module each (header, footer, product card, product image, stars, pagination, breadcrumbs, status badge, error page, palette, icons).
- `src/data/`: the dataset and the startup indexes, copied from the reference app. The store is shared through the router context (`.with(store)`, read with `context::<Store>(cx)`).
- `src/format.rs`, `src/cart.rs`: money, count and date formatting, and the cart cookie and signal.

## Judgment calls

- **Routing and pages.** Pages are `#[page]` functions found by `module_router!()`, the API endpoints are `#[route]`s, and every link is built with `href!` (with `.query(...)` for listing links), as the [href guide](../../../docs/router/href.md) asks. The listing reads its parameters with a `#[query_params]` struct, which turns empty values into `None`.
- **Layout and errors.** One `#[layout]` renders the document. It wraps the page in an `error_boundary` that turns a `NotFoundError` or the app's `InvalidRequestError` into the error page and sets the status code, the pattern from the [error guide](../../../docs/router/error.md). Unknown URLs reach the layout through a catch-all page, which is what `not_found!()` expands to plus a page title.
- **Titles.** Page titles use [`Metadata`](../../../docs/metadata.md) and `topcoat::metadata::tags()`. The listing and detail pages compute theirs with `Metadata::from_fn`, which also picks the error title when the lookup fails, because the layout renders `<head>` before the page body. `tags()` also emits Topcoat's default `og:type`, `og:title` and `twitter:card` tags.
- **Components.** Every reusable piece is a `#[component]`, per the docs ("call a component, not a helper"). Loops over components use `#[key(...)]`.
- **Classes.** Fixed class lists are plain `class="..."` attributes. Class lists picked by data or state use `class!` and `StaticClass` constants, so their literals are escaped at compile time, for example `class!("size-3 rounded-full ring-1 ring-black/10", palette.accent)` and `class!("text-xs", "text-teal-700" if selected else "text-stone-400")`.
- **Formatting.** Dates use `time`'s own formatter: `format_description!` constants and `.format(...)` (SPEC rule 9). Money and thousands separators are small hand-written `Display` types, since Rust's standard library has no such formatter. `view!` renders strings and numbers but not `Display` values, so these are rendered with `.to_string()`, as the repository's admin demo does.
- **Tailwind.** `topcoat::tailwind::BuildConfig` runs the shell's `tailwindcss` (`.executable("tailwindcss")`) so every app uses the same Tailwind release, with an input that scans only `src/` like the reference. The stylesheet is written to `target/tailwind.css` and declared with `asset!` instead of the default `stylesheet!()` (`$OUT_DIR/tailwind.css`): an asset's id includes its path, `OUT_DIR` changes with cargo features, and `topcoat asset bundle` has no `--features` flag, so a bundle built by the CLI would not match the binary built with an allocator feature.
- **Assets.** The stylesheet and the runtime script are served by Topcoat's asset bundle under content-hashed `/_topcoat/assets/...` URLs.
- **Cookies.** `CookieLayer` and `cookies(cx)` read `cart_count`, as in the [cookie guide](../../../docs/cookie.md).
- **Interactivity.** Both pieces use Topcoat's [runtime](../../../docs/runtime.md): signals, `@click` handlers and `:class` / `:aria-expanded` / `:disabled` bindings. The cart count is one signal, declared by the `cart_signal` helper from the layout and the detail page; both get the same signal because the helper's call site and the request context are the same. Writing the cookie uses `raw!` JavaScript, since the expression vocabulary has no cookie API. The add-to-cart handler adds to the cart signal (initialized from the cookie on the server) instead of re-reading `document.cookie`; the result only differs if another tab changed the cookie after the page loaded.
- **API errors.** A `#[layer]` on `/api` turns the app's `ApiError` into the `{"error": "..."}` response. The cart route takes the raw `Body` and runs the `Json` extractor itself, because the extractor rejects bad bodies with 400 and the spec asks for 422.
- **Cookie write.** The `raw!` call reads its cookie name and attributes from local bindings. Written as one string literal, the call is too long for one line, and `topcoat fmt` then wraps it with a trailing comma that `raw!` rejects.
- **Dev script.** The layout includes `topcoat::dev::script()` like the `topcoat new` template. It renders nothing outside `topcoat dev`.
