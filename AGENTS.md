# Agent instructions

Before making code or documentation changes, ensure you have read `./STYLE.md`.

## Project structure

Topcoat is a Cargo workspace. The framework crates live in `crates/`, runnable example apps in `examples/`, and the prose guides in `docs/`.

`crates/topcoat` is the user-facing **facade** crate. It re-exports everything through feature-gated modules. Application code depends on this crate only; everything below is an implementation detail reached through it.

- `topcoat-core` (+ `macro/`): foundations shared by the other crates: the `Error`/`Result` types and the request context (`Cx`, `app_context`, `request_context`). Its macro crate provides `#[memoize]`, and its `grammar/` crate holds the pretty-printer backing `topcoat fmt`'s macro-body formatting (behind the `pretty` feature).
- `topcoat-view` (+ `macro/`): the `view!`, `attributes!`, and `class!` macros, the `#[component]` macro, and the runtime `View`/`Attributes`/`Class` types.
- `topcoat-router` (+ `macro/`): `Router`, the `#[page]`/`#[layout]`/`#[route]` macros, `module_router!`, and `#[path_param]`/`#[query_params]`.
- `topcoat-runtime` (+ `macro/`): the client-side interactive runtime (signals, event handlers, bind attributes, the `expr!` macro) and the injected browser script.
- `topcoat-asset`: the `asset!` macro and `AssetBundle` for declaring and serving content-hashed static files.
- `topcoat-cookie`: the cookie jar, `cookie!` macro, signed/private jars, and `CookieStore<T>`.
- `topcoat-tailwind`: the build-script wrapper around the standalone Tailwind CLI.
- `topcoat-cli`: the `topcoat` binary (`dev`, `fmt`, `asset` subcommands).

The domain crates that back proc-macros follow a common split: an `ast` module (behind the `parsing` feature, used at compile time by the sibling `macro/` crate) and a `runtime` module (the code the generated output calls into at run time).

## Documentation

The `docs/` directory contains the framework's user-facing guides. Consult the relevant one before working on a feature in that area.

### Getting started

- [`crates/topcoat/docs/getting_started.md`](crates/topcoat/docs/getting_started.md): Creating a new project, installing the `topcoat` CLI, and running the dev server.

### Routing

- [`crates/topcoat/docs/router.md`](crates/topcoat/docs/router.md): The `Router` primitive: registering `#[page]`, `#[layout]`, and `#[route]` items manually or via `.discover()`, and how layouts nest by path prefix.
- [`crates/topcoat-router/docs/module_router.md`](crates/topcoat-router/docs/module_router.md): `module_router!`, which derives routes from the module tree (kebab-cased segments, `segment!` overrides, `_`-prefixed groups).

### Views and components

- [`crates/topcoat-view/macro/docs/view.md`](crates/topcoat-view/macro/docs/view.md): The `view!` macro: HTML-like templating syntax, expression interpolation, control flow (`if`/`for`/`match`/`let`), components, and conditional attributes.
- [`crates/topcoat-view/macro/docs/component.md`](crates/topcoat-view/macro/docs/component.md): The `#[component]` macro: defining components, props, child content, generics, and the `cx` parameter.
- [`crates/topcoat-view/macro/docs/attributes.md`](crates/topcoat-view/macro/docs/attributes.md): The `attributes!` macro and the runtime `Attributes` value for building/forwarding attribute collections.
- [`crates/topcoat-view/macro/docs/class.md`](crates/topcoat-view/macro/docs/class.md): The `class!` macro: assembling a space-separated class list from static and conditional entries.

### Request context and state

- [`crates/topcoat/docs/context.md`](crates/topcoat/docs/context.md): The request context `Cx`: router request helpers, path/query helpers, state accessors, and request body parsing.
- [`crates/topcoat/docs/app_context.md`](crates/topcoat/docs/app_context.md): App context: registering long-lived values with `.app_context(value)` and reading them with `app_context::<T>(cx)`.
- [`crates/topcoat-core/macro/docs/memoization.md`](crates/topcoat-core/macro/docs/memoization.md): `#[memoize]` for per-request caching of function results keyed by arguments.
- [`crates/topcoat/docs/functions_not_middlewares.md`](crates/topcoat/docs/functions_not_middlewares.md): The framework's philosophy: prefer composable `cx: &Cx` functions over middleware/extractors for auth and request-scoped data.
- [`crates/topcoat/docs/cookie.md`](crates/topcoat/docs/cookie.md): Cookies: the request-scoped jar (`cookies(cx)`), the `cookie!` macro, attribute defaults, name prefixes, signed/private cookies, and typed `CookieStore<T>`.

### Assets and styling

- [`crates/topcoat/docs/asset.md`](crates/topcoat/docs/asset.md): Declaring static files with `asset!`, content-hashed URLs, and loading the asset bundle on the router.
- [`crates/topcoat/docs/tailwind.md`](crates/topcoat/docs/tailwind.md): The Tailwind integration: a build-script wrapper around the standalone Tailwind CLI served as a Topcoat asset.

### Tooling

- [`crates/topcoat-cli/docs/fmt.md`](crates/topcoat-cli/docs/fmt.md): `topcoat fmt`, which formats Topcoat macro bodies (like `view!`) alongside `rustfmt`, plus editor integration.

## Safety

This project only uses safe code. Unsafe is not allowed.
