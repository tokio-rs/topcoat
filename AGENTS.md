# Agent instructions

## Project structure

Topcoat is a Cargo workspace. The framework crates live in `crates/`, small single-feature examples in `examples/`, complete demo applications in `demos/`, and the prose guides in the top-level `docs/` directory.

`crates/topcoat` is the user-facing **facade** crate. It re-exports everything through feature-gated modules. Application code depends on this crate only; everything below is an implementation detail reached through it.

- `topcoat-core`: foundations shared by the other crates: the `Error`/`Result` types and the request context (`Cx`, `app_context`, `request_context`). Its macro crate provides `#[memoize]`, and its grammar crate holds the pretty-printer backing `topcoat fmt`'s macro-body formatting.
- `topcoat-view`: the `view!`, `live!`, `emit!`, `attributes!`, and `class!` macros, the `#[component]` macro, and the runtime `View`/`Attributes`/`Class` types.
- `topcoat-router`: `Router`, the `#[page]`/`#[layout]`/`#[route]` macros, `module_router!`, `path_param!`, and `#[query_params]`.
- `topcoat-runtime`: the client-side interactive runtime (signals, event handlers, bind attributes, the `expr!` macro) and the injected browser script.
- `topcoat-font`: the `font!` and `font_face!` macros and the Fontsource integration for bundling and serving web fonts.
- `topcoat-icon`: the `icon` component and the Iconify integration for vendoring icon sets into a project.
- `topcoat-asset`: the `asset!` macro and `AssetBundle` for declaring and serving content-hashed static files.
- `topcoat-cookie`: the cookie jar, `cookie!` macro, signed/private jars, and `CookieStore<T>`.
- `topcoat-session`: bring-your-own-storage session authentication: the token/hash model, the session lifecycle, and origin checking.
- `topcoat-mail`: the `Mail` type and `mail!` macro for declaring mail, and its delivery through pluggable transports (SMTP, file, in-memory).
- `topcoat-htmx`, `topcoat-alpine-ajax`, and `topcoat-datastar`: request and response helpers for those client libraries.
- `topcoat-tailwind`: the build-script wrapper around the standalone Tailwind CLI.
- `topcoat-ui` (+ `registry/`): the component registry behind `topcoat ui`, which copies component source into a project.
- `topcoat-cli`: the `topcoat` binary. Each subcommand has its own module under `src/`.

A crate that backs proc-macros comes as a trio. The base crate holds the runtime types the generated code calls into. Its `grammar/` crate parses the macro body and generates the code, and is only used at compile time. Its `macro/` crate is a thin proc-macro entry point over `grammar/`. Where a macro body is formattable, the `grammar/` crate's `pretty` feature adds the pretty-printer `topcoat fmt` uses.

## Documentation

The top-level `docs/` directory is the source of truth for guides. Its structure follows the modules, with `router.md` beside `router/`, `router/module.md` for module routing, and `router/content/sse.md` for server-sent events. Crate-local `docs/` files are relative symlinks to those sources and omit the crate's module prefix, such as `crates/topcoat-router/docs/content/sse.md`. The facade crate keeps the full module paths. Keep `include_str!` paths inside each crate so its published package includes the documentation. Edit the top-level source, and add a crate-local symlink when embedding a new guide into the API docs. Consult the relevant guide before working on a feature. The index below covers the main guides; check `docs/` for anything not listed here.

### Getting started

- [`docs/getting_started.md`](docs/getting_started.md): Creating a new project, installing the `topcoat` CLI, and running the dev server.

### Routing

- [`docs/router.md`](docs/router.md): The `Router` primitive: registering `#[page]`, `#[layout]`, and `#[route]` items manually or via `.discover()`, and how layouts nest by path prefix.
- [`docs/router/module.md`](docs/router/module.md): `module_router!`, which derives routes from the module tree (kebab-cased segments, `segment!` overrides, `_`-prefixed groups).
- [`docs/router/error.md`](docs/router/error.md): Router errors: the status-code constructors, the `RouterErrorExt` conversions from `Option`/`Result`, catching an error in an outer handler, and internal rewrites.
- [`docs/router/tower.md`](docs/router/tower.md): The tower bridge: `TowerRoute` for mounting a tower service as a route, `TowerLayer` for running tower middleware as a layer, and `TowerService` for serving the router as a tower service.
- [`docs/router/content.md`](docs/router/content.md): Request and response bodies: `FromRequest` extractors, `IntoResponse` return values, and an overview of the content types below.
- [`docs/router/content/websocket.md`](docs/router/content/websocket.md): WebSockets (behind the `websocket` feature): the `WebSocketUpgrade` extractor, exchanging `Message`s over a `WebSocket`, subprotocol negotiation, and connection limits.
- [`docs/router/content/sse.md`](docs/router/content/sse.md): Server-sent events (behind the `sse` feature): the `Sse` streaming response, building `Event`s, keep-alive events for idle streams, and resuming from `Last-Event-ID`.
- [`docs/router/content/multipart.md`](docs/router/content/multipart.md): Multipart form data (behind the `multipart` feature): the `Multipart` extractor and reading uploaded `Field`s.
- [`docs/router/content/sitemap.md`](docs/router/content/sitemap.md): XML sitemaps (behind the `sitemap` feature): the `Sitemap` response, entry fields, base URL resolution, and serving `/sitemap.xml`.
- [`docs/router/`](docs/router): A reference page per routing macro, covering the attributes each one accepts.

### Views and components

- [`docs/view/view.md`](docs/view/view.md): The `view!` macro: HTML-like templating syntax, expression interpolation, control flow (`if`/`for`/`match`/`let`), components, and conditional attributes.
- [`docs/view/component.md`](docs/view/component.md): The `#[component]` macro: defining components, props, child content, generics, and the `cx` parameter.
- [`docs/view/live.md`](docs/view/live.md): The `live!` and `emit!` macros: live regions that stream replacement content into a page, and the `suspense`/`error_boundary` components built on them.
- [`docs/view/attributes.md`](docs/view/attributes.md): The `attributes!` macro and the runtime `Attributes` value for building/forwarding attribute collections.
- [`docs/view/class.md`](docs/view/class.md): The `class!` macro: assembling a space-separated class list from static and conditional entries.
- [`docs/view/props.md`](docs/view/props.md): The `Props` derive macro, which generates a props struct's typestate builder.

### UI components

- [`docs/ui.md`](docs/ui.md): The `topcoat ui` workflow: initializing a package with a theme, adding, updating, and removing vendored components, and writing custom registries.

### Client reactivity

- [`docs/runtime.md`](docs/runtime.md): The runtime guide: signals, runtime expressions, `@` event handlers, `:` bind attributes, and how procedures and shards fit in.
- [`docs/runtime/expr.md`](docs/runtime/expr.md): The `expr!` macro: the dual Rust/JavaScript expression language, its shared vocabulary, captured variables, and `raw!`.
- [`docs/runtime/procedure.md`](docs/runtime/procedure.md): The `#[procedure]` macro: async server functions callable from runtime expressions.
- [`docs/runtime/shard.md`](docs/runtime/shard.md): The `#[shard]` macro: components that re-render on the server when their runtime expression arguments change.

### Request context and state

- [`docs/context.md`](docs/context.md): The request context `Cx`: router request helpers, path/query helpers, state accessors, and request body parsing.
- [`docs/context/app_context.md`](docs/context/app_context.md): App context: registering long-lived values with `.app_context(value)` and reading them with `app_context::<T>(cx)`.
- [`docs/context/memoize.md`](docs/context/memoize.md): `#[memoize]` for per-request caching of function results keyed by arguments.
- [`docs/context/functions_not_middlewares.md`](docs/context/functions_not_middlewares.md): The framework's philosophy: prefer composable `cx: &Cx` functions over middleware/extractors for auth and request-scoped data.
- [`docs/cookie.md`](docs/cookie.md): Cookies: the request-scoped jar (`cookies(cx)`), the `cookie!` macro, attribute defaults, name prefixes, signed/private cookies, and typed `CookieStore<T>`.
- [`docs/session.md`](docs/session.md): Sessions: bring-your-own-storage session authentication -- the token/hash model, the `start`/`stop` lifecycle, sliding expiration and rotation, and custom token stores.

### Assets and styling

- [`docs/asset.md`](docs/asset.md): Declaring static files with `asset!`, content-hashed URLs, and loading the asset bundle on the router.
- [`docs/tailwind.md`](docs/tailwind.md): The Tailwind integration: a build-script wrapper around the standalone Tailwind CLI served as a Topcoat asset.
- [`docs/font.md`](docs/font.md): Declaring web fonts with `font!` and `font_face!`, serving them as assets, and pulling families from Fontsource.
- [`docs/icon.md`](docs/icon.md): The `icon` component and the Iconify integration for vendoring icon sets at build time.

### Mail

- [`docs/mail.md`](docs/mail.md): Mail: declaring a `Mail` with the `mail!` macro, the transports (SMTP, file, in-memory), and delivering with `send`.

### Client library integrations

- [`docs/htmx.md`](docs/htmx.md): htmx: reading its request headers and setting its response headers from a handler.
- [`docs/alpine_ajax.md`](docs/alpine_ajax.md): Alpine AJAX: reading its request headers to render partial responses.
- [`docs/datastar.md`](docs/datastar.md): Datastar: reading the signals sent with a request and patching elements and signals over server-sent events.

### Tooling

- [`docs/cli/fmt.md`](docs/cli/fmt.md): `topcoat fmt`, which formats Topcoat macro bodies (like `view!`) alongside `rustfmt`, plus editor integration.

## Safety

This project only uses safe code. Unsafe is not allowed.
