# Icon support: remaining design

This document describes the parts of icon support that are designed but not
yet implemented on this branch, primarily the Iconify integration.

## Already on this branch

- `topcoat-view`: const `View::unescaped(Unescaped<&'static str>)` and const
  `Unescaped::new_unchecked`, the `Length` and `svg::ViewBox` runtime types,
  and self-closing tags in the `view!` parser (own `SelfClosingTag` ast type,
  `/>` preserved verbatim in output, works with dynamic element names).
- `topcoat-icon`: `IconData` holding a `ViewBox` and a body `View`, with a
  const `IconData::unescaped` constructor for `const`/`static` icons.
- Facade: the `topcoat::icon` module with the `icon` component (defaults to a
  `1em` square, `aria-hidden` unless a `label` is passed, forwards `attrs` to
  the `<svg>` element), behind the `icon` feature. The `icon-iconify` feature
  and the empty `topcoat-icon-iconify` (features `parsing`/`build` declared)
  and `topcoat-icon-iconify-macro` (stub) crates exist but contain no code.
- `examples/icon`: demos hand-written const icons and `view!`-built bodies.

## Iconify pipeline overview

The integration is split into two phases so that proc macros never touch the
network and builds work offline:

1. The consumer's `build.rs` stages Iconify icon set JSON into
   `$OUT_DIR/topcoat-icon-iconify/<prefix>.json`, either from files the user
   vendors or by downloading them from jsDelivr.
2. The `iconify::include!` and `iconify::iconify_icon!` macros read the staged
   JSON at expansion time (via the `OUT_DIR` env var, which is set for crates
   that have a build script) and expand to const `IconData` values.

Consumer setup, mirroring the Tailwind integration:

```toml
[dependencies]
topcoat = { version = "...", features = ["icon-iconify"] }

[build-dependencies]
topcoat = { version = "...", default-features = false, features = ["icon-iconify"] }
```

```rust
// build.rs
fn main() {
    topcoat::icon::iconify::Sets::new()
        .download("lucide")
        .download_version("mdi", "1.30.0")
        .vendor("vendor/simple-icons.json")
        .stage()
        .unwrap();
}
```

```rust
use topcoat::icon::iconify;

iconify::include!("lucide");     // module with consts: lucide::TRASH_2
iconify::include!("mdi:*");      // inlines all consts into the current scope
iconify::include!("mdi:delete"); // a single const: DELETE

const PENCIL: IconData = iconify::iconify_icon!("lucide:pencil"); // expression
```

## Staging build tools (`topcoat-icon-iconify`, `build` feature)

`Sets` is a builder used from `build.rs`:

- `Sets::new()`
- `.download(set)`: fetch the latest version of a set.
- `.download_version(set, version)`: fetch a pinned version.
- `.vendor(path)`: use a local Iconify JSON file, relative to
  `CARGO_MANIFEST_DIR`.
- `.stage()`: write every set to `$OUT_DIR/topcoat-icon-iconify/<prefix>.json`
  and return a `Result` with a `thiserror` error enum (modeled on the
  `topcoat-tailwind` build error).

Sourcing and caching:

- Downloads use `ureq` against
  `https://cdn.jsdelivr.net/npm/@iconify-json/{set}@{version}/icons.json`,
  with the `latest` tag for `.download()`.
- Pinned downloads write a `<prefix>.version` sidecar and are skipped when the
  sidecar matches the requested version. Latest downloads are skipped whenever
  the staged file already exists, so builds stay offline after the first
  fetch; refresh by cleaning the build directory or pinning a version.
- `stage()` prints `cargo::rerun-if-changed=build.rs` plus one
  `cargo::rerun-if-changed=<path>` per vendored file, so vendored edits
  restage but ordinary source edits do not rerun the script.

Staging normalizes rather than copying verbatim, so the macro side stays
simple and JSON problems surface at build time with good errors:

- Per-icon geometry is filled in: `width`/`height` fall back to the set-level
  values and then to Iconify's default of `16`; `left`/`top` default to `0`.
- Alias parent chains are resolved to their terminal parent. Aliases that
  carry transforms (`rotate`, `hFlip`, `vFlip`) are dropped in v1; supporting
  them would mean wrapping the body in a `<g transform>`.
- The normalized shape is `{ prefix, icons: { name: { body, left, top, width,
  height, hidden } }, aliases: { name: parent } }`.

A shared serde model module is compiled under both the `build` and `parsing`
features: the normalized `IconSet` used by both sides, plus a raw
Iconify-format type private to the build half.

## Macros (`parsing` feature + `topcoat-icon-iconify-macro`)

Per repo convention the proc-macro crate stays thin: it parses into ast types
defined in `topcoat-icon-iconify` behind the `parsing` feature and emits them
with `quote! { #parsed }`. The macros are exported as `include` and
`iconify_icon` and re-exported through `topcoat::icon::iconify`.

Expansion:

- The staged file is located through `env::var("OUT_DIR")`. A missing var
  produces a compile error explaining that the consuming crate needs a
  `build.rs` that stages icon sets (with a snippet). A missing prefix file
  lists the prefixes that are staged, plus the same hint.
- Parsed sets are cached per process in a static map keyed by path, since the
  proc-macro server expands many invocations in one process.
- Each icon expands to a const-evaluable expression using full facade paths:

```rust
::topcoat::icon::IconData::unescaped(
    ::topcoat::view::svg::ViewBox::new(0.0, 0.0, 24.0, 24.0),
    ::topcoat::view::Unescaped::new_unchecked("<path .../>"),
)
```

  with the view box built from the staged `left`/`top`/`width`/`height`.

The three `include!` forms:

- `include!("mdi")` generates `mod mdi { ... }` containing a `pub const` per
  icon. The module itself is private to the invocation site; an optional
  leading visibility (`include!(pub(crate) "mdi")`) applies to the module.
- `include!("mdi:*")` inlines the consts into the current scope with no
  module; the optional visibility applies to each const.
- `include!("mdi:delete")` generates the single const `DELETE`.
- Globs skip icons marked `hidden` but include resolved aliases; naming an
  icon explicitly works even when it is hidden.
- `iconify_icon!("mdi:delete")` is the expression form and stays usable in
  `const` contexts.

Const naming is deterministic:

- Icon names are kebab-case and map to SCREAMING_SNAKE_CASE (`-` becomes
  `_`): `trash-2` -> `TRASH_2`.
- Names starting with a digit get a leading underscore: `123` -> `_123`,
  `24-hours` -> `_24_HOURS`, `2fa` -> `_2FA`. Iconify names never contain
  underscores, so this cannot collide.
- Module names from `include!("mdi")` follow the same rule in lowercase.

Diagnostics: unknown sets and icons produce a compile error spanned to the
string literal, with up to three near-miss suggestions by edit distance.
rust-analyzer resolves the macros as long as build scripts are enabled (the
default), because it then provides `OUT_DIR` to proc macros.

## Facade wiring still missing

- `topcoat::icon::iconify` currently re-exports only the macro crate; it also
  needs `pub use topcoat_icon_iconify::*;` for the build API.
- `topcoat-icon-iconify` keeps no default features. The facade `icon-iconify`
  feature additionally enables `topcoat-icon-iconify/build` so `Sets` is
  reachable from a consumer's `build.rs` through the facade, while the macro
  crate depends on it with only `parsing`.

## Testing plan

- Unit tests in `topcoat-icon-iconify` for const naming, alias resolution,
  normalization, and codegen. Codegen tests take an `&IconSet` and the
  request directly so they never need `OUT_DIR`.
- End to end: extend `examples/icon` with a `build.rs` that stages a small
  vendored fixture set, and use the `include!` forms in `main.rs`. Building
  the workspace then exercises the whole pipeline without network access.
- Run the full workspace test suite, including the new self-closing tag tests
  in `topcoat-view`.

## Deferred

- An `icon!` macro for authoring icons in `view!` syntax: intentionally
  skipped for v1; `IconData::new` with a `view!` body covers the use case.
- Transform aliases (rotate/flip), per the staging section.
- User-facing documentation (`docs/` guide and module docs) until the API
  settles.
