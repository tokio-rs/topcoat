# Agent instructions

Before making code changes, ensure you have read `./STYLE.md`.

## Project structure

Topcoat is a Cargo workspace. The framework crates live in `crates/`, runnable example apps in `examples/`, and the prose guides in `docs/`.

`crates/topcoat` is the user-facing **facade** crate. It re-exports everything through feature-gated modules. Application code depends on this crate only; everything below is an implementation detail reached through it.

- `topcoat-core` (+ `macro/`) — foundations shared by the other crates: the `Error`/`Result` types and the request context (`Cx`, `app_state`, `request_state`). Its macro crate provides `#[memoize]`.
- `topcoat-view` (+ `macro/`) — the `view!` and `attributes!` macros, the `#[component]` macro, and the runtime `View`/`Attributes` types.
- `topcoat-router` (+ `macro/`) — `Router`, the `#[page]`/`#[layout]`/`#[route]` macros, `module_router!`, and `#[path_param]`/`#[query_params]`.
- `topcoat-runtime` (+ `macro/`) — the client-side interactive runtime (signals, event handlers, bind attributes, the `expr!` macro) and the injected browser script.
- `topcoat-asset` — the `asset!` macro and `AssetBundle` for declaring and serving content-hashed static files.
- `topcoat-cookie` — the cookie jar, `cookie!` macro, signed/private jars, and `CookieStore<T>`.
- `topcoat-tailwind` — the build-script wrapper around the standalone Tailwind CLI.
- `topcoat-pretty` — the pretty-printer backing `topcoat fmt`'s macro-body formatting.
- `topcoat-cli` — the `topcoat` binary (`dev`, `fmt`, `asset` subcommands).

The domain crates that back proc-macros follow a common split: an `ast` module (behind the `parsing` feature, used at compile time by the sibling `macro/` crate) and a `runtime` module (the code the generated output calls into at run time).

## Documentation

The `docs/` directory contains the framework's user-facing guides. Consult the relevant one before working on a feature in that area.

### Getting started

- [`docs/getting_started.md`](docs/getting_started.md) — Creating a new project, installing the `topcoat` CLI, and running the dev server.

### Routing

- [`docs/router.md`](docs/router.md) — The `Router` primitive: registering `#[page]`, `#[layout]`, and `#[route]` items manually or via `.discover()`, and how layouts nest by path prefix.
- [`docs/module_router.md`](docs/module_router.md) — `module_router!`, which derives routes from the module tree (kebab-cased segments, `segment!` overrides, `_`-prefixed groups).
- [`docs/path_and_query_params.md`](docs/path_and_query_params.md) — `#[path_param]` and `#[query_params]` for reading typed values out of the URL via `T::of(cx)`.
- [`docs/request_response.md`](docs/request_response.md) — Request body extractors (`Json`, `Form`, `Multipart`, raw bodies) and response conversion (`IntoResponse`), including custom `FromRequest`/`IntoResponse`.

### Views and components

- [`docs/view.md`](docs/view.md) — The `view!` macro: HTML-like templating syntax, expression interpolation, control flow (`if`/`for`/`match`/`let`), components, and conditional attributes.
- [`docs/component.md`](docs/component.md) — The `#[component]` macro: defining components, props, child content, generics, and the `cx` parameter.
- [`docs/attributes.md`](docs/attributes.md) — The `attributes!` macro and the runtime `Attributes` value for building/forwarding attribute collections.

### Request context and state

- [`docs/context.md`](docs/context.md) — The request context `Cx`: router request helpers, path/query helpers, state accessors, and the Axum extractor escape hatch.
- [`docs/app_state.md`](docs/app_state.md) — App state: registering long-lived values with `.app_state(value)` and reading them with `app_state::<T>(cx)`.
- [`docs/memoization.md`](docs/memoization.md) — `#[memoize]` for per-request caching of function results keyed by arguments.
- [`docs/functions_not_middlewares.md`](docs/functions_not_middlewares.md) — The framework's philosophy: prefer composable `cx: &Cx` functions over middleware/extractors for auth and request-scoped data.
- [`docs/cookies.md`](docs/cookies.md) — Cookies: the request-scoped jar (`cookies(cx)`), the `cookie!` macro, attribute defaults, name prefixes, signed/private cookies, and typed `CookieStore<T>`.

### Assets and styling

- [`docs/assets.md`](docs/assets.md) — Declaring static files with `asset!`, content-hashed URLs, and loading the asset bundle on the router.
- [`docs/tailwind.md`](docs/tailwind.md) — The Tailwind integration: a build-script wrapper around the standalone Tailwind CLI served as a Topcoat asset.

### Tooling

- [`docs/source_formatting.md`](docs/source_formatting.md) — `topcoat fmt`, which formats Topcoat macro bodies (like `view!`) alongside `rustfmt`, plus editor integration.
