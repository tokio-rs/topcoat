# Agent instructions

## Project structure

Topcoat is a Cargo workspace. The framework crates live in `crates/`, runnable example apps in `examples/`, and the prose guides in a `docs/` directory inside the crate they document.

`crates/topcoat` is the user-facing **facade** crate. It re-exports everything through feature-gated modules. Application code depends on this crate only; everything below is an implementation detail reached through it.

- `topcoat-core`: foundations shared by the other crates: the `Error`/`Result` types and the request context (`Cx`, `app_context`, `request_context`). Its macro crate provides `#[memoize]`, and its grammar crate holds the pretty-printer backing `topcoat fmt`'s macro-body formatting.
- `topcoat-view`: the `view!`, `attributes!`, and `class!` macros, the `#[component]` macro, and the runtime `View`/`Attributes`/`Class` types.
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

Each crate's `docs/` directory holds the user-facing guides for that crate, embedded into the API docs with `#![doc = include_str!(...)]`. Consult the relevant one before working on a feature in that area. The index below covers the main guides; when working on a crate, check its own `docs/` directory for anything not listed here.

### Getting started

- [`crates/topcoat/docs/getting_started.md`](crates/topcoat/docs/getting_started.md): Creating a new project, installing the `topcoat` CLI, and running the dev server.

### Routing

- [`crates/topcoat/docs/router.md`](crates/topcoat/docs/router.md): The `Router` primitive: registering `#[page]`, `#[layout]`, and `#[route]` items manually or via `.discover()`, and how layouts nest by path prefix.
- [`crates/topcoat-router/docs/module_router.md`](crates/topcoat-router/docs/module_router.md): `module_router!`, which derives routes from the module tree (kebab-cased segments, `segment!` overrides, `_`-prefixed groups).
- [`crates/topcoat-router/docs/error.md`](crates/topcoat-router/docs/error.md): Router errors: the status-code constructors, the `RouterErrorExt` conversions from `Option`/`Result`, and catching an error in an outer handler.
- [`crates/topcoat-router/docs/tower.md`](crates/topcoat-router/docs/tower.md): The tower bridge: `TowerRoute` for mounting a tower service as a route and `TowerLayer` for running tower middleware as a layer.
- [`crates/topcoat-router/docs/content.md`](crates/topcoat-router/docs/content.md): Request and response bodies: `FromRequest` extractors, `IntoResponse` return values, and an overview of the content types below.
- [`crates/topcoat-router/docs/content/websocket.md`](crates/topcoat-router/docs/content/websocket.md): WebSockets (behind the `websocket` feature): the `WebSocketUpgrade` extractor, exchanging `Message`s over a `WebSocket`, subprotocol negotiation, and connection limits.
- [`crates/topcoat-router/docs/content/sse.md`](crates/topcoat-router/docs/content/sse.md): Server-sent events (behind the `sse` feature): the `Sse` streaming response, building `Event`s, keep-alive events for idle streams, and resuming from `Last-Event-ID`.
- [`crates/topcoat-router/docs/content/multipart.md`](crates/topcoat-router/docs/content/multipart.md): Multipart form data (behind the `multipart` feature): the `Multipart` extractor and reading uploaded `Field`s.
- [`crates/topcoat-router/macro/docs/`](crates/topcoat-router/macro/docs): A reference page per routing macro, covering the attributes each one accepts.

### Views and components

- [`crates/topcoat-view/macro/docs/view.md`](crates/topcoat-view/macro/docs/view.md): The `view!` macro: HTML-like templating syntax, expression interpolation, control flow (`if`/`for`/`match`/`let`), components, and conditional attributes.
- [`crates/topcoat-view/macro/docs/component.md`](crates/topcoat-view/macro/docs/component.md): The `#[component]` macro: defining components, props, child content, generics, and the `cx` parameter.
- [`crates/topcoat-view/macro/docs/attributes.md`](crates/topcoat-view/macro/docs/attributes.md): The `attributes!` macro and the runtime `Attributes` value for building/forwarding attribute collections.
- [`crates/topcoat-view/macro/docs/class.md`](crates/topcoat-view/macro/docs/class.md): The `class!` macro: assembling a space-separated class list from static and conditional entries.
- [`crates/topcoat-view/macro/docs/props.md`](crates/topcoat-view/macro/docs/props.md): The `props!` macro for building a component's props value.

### UI components

- [`crates/topcoat/docs/ui.md`](crates/topcoat/docs/ui.md): The `topcoat ui` workflow: initializing a package with a theme, adding, updating, and removing vendored components, and writing custom registries.

### Client reactivity

- [`crates/topcoat/docs/runtime.md`](crates/topcoat/docs/runtime.md): The runtime guide: signals, runtime expressions, `@` event handlers, `:` bind attributes, and how procedures and shards fit in.
- [`crates/topcoat-runtime/macro/docs/expr.md`](crates/topcoat-runtime/macro/docs/expr.md): The `expr!` macro: the dual Rust/JavaScript expression language, its shared vocabulary, captured variables, and `raw!`.
- [`crates/topcoat-runtime/macro/docs/procedure.md`](crates/topcoat-runtime/macro/docs/procedure.md): The `#[procedure]` macro: async server functions callable from runtime expressions.
- [`crates/topcoat-runtime/macro/docs/shard.md`](crates/topcoat-runtime/macro/docs/shard.md): The `#[shard]` macro: components that re-render on the server when their runtime expression arguments change.

### Request context and state

- [`crates/topcoat/docs/context.md`](crates/topcoat/docs/context.md): The request context `Cx`: router request helpers, path/query helpers, state accessors, and request body parsing.
- [`crates/topcoat/docs/app_context.md`](crates/topcoat/docs/app_context.md): App context: registering long-lived values with `.app_context(value)` and reading them with `app_context::<T>(cx)`.
- [`crates/topcoat-core/macro/docs/memoize.md`](crates/topcoat-core/macro/docs/memoize.md): `#[memoize]` for per-request caching of function results keyed by arguments.
- [`crates/topcoat/docs/functions_not_middlewares.md`](crates/topcoat/docs/functions_not_middlewares.md): The framework's philosophy: prefer composable `cx: &Cx` functions over middleware/extractors for auth and request-scoped data.
- [`crates/topcoat/docs/cookie.md`](crates/topcoat/docs/cookie.md): Cookies: the request-scoped jar (`cookies(cx)`), the `cookie!` macro, attribute defaults, name prefixes, signed/private cookies, and typed `CookieStore<T>`.
- [`crates/topcoat/docs/session.md`](crates/topcoat/docs/session.md): Sessions: bring-your-own-storage session authentication -- the token/hash model, the `start`/`stop` lifecycle, sliding expiration and rotation, and custom token stores.

### Assets and styling

- [`crates/topcoat/docs/asset.md`](crates/topcoat/docs/asset.md): Declaring static files with `asset!`, content-hashed URLs, and loading the asset bundle on the router.
- [`crates/topcoat/docs/tailwind.md`](crates/topcoat/docs/tailwind.md): The Tailwind integration: a build-script wrapper around the standalone Tailwind CLI served as a Topcoat asset.
- [`crates/topcoat/docs/font.md`](crates/topcoat/docs/font.md): Declaring web fonts with `font!` and `font_face!`, serving them as assets, and pulling families from Fontsource.
- [`crates/topcoat/docs/icon.md`](crates/topcoat/docs/icon.md): The `icon` component and the Iconify integration for vendoring icon sets at build time.

### Mail

- [`crates/topcoat/docs/mail.md`](crates/topcoat/docs/mail.md): Mail: declaring a `Mail` with the `mail!` macro, the transports (SMTP, file, in-memory), and delivering with `send`.

### Client library integrations

- [`crates/topcoat/docs/htmx.md`](crates/topcoat/docs/htmx.md): htmx: reading its request headers and setting its response headers from a handler.
- [`crates/topcoat/docs/alpine-ajax.md`](crates/topcoat/docs/alpine-ajax.md): Alpine AJAX: reading its request headers to render partial responses.
- [`crates/topcoat/docs/datastar.md`](crates/topcoat/docs/datastar.md): Datastar: reading the signals sent with a request and patching elements and signals over server-sent events.

### Tooling

- [`crates/topcoat-cli/docs/fmt.md`](crates/topcoat-cli/docs/fmt.md): `topcoat fmt`, which formats Topcoat macro bodies (like `view!`) alongside `rustfmt`, plus editor integration.

## Safety

This project only uses safe code. Unsafe is not allowed.
