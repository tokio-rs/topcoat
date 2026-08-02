---
name: topcoat-ui
description: Build, style, review, and debug user interfaces in Topcoat Rust applications. Use for view! templates, components, attributes, classes, CSS, Tailwind, Topcoat UI, assets, fonts, Fontsource, icons, Iconify, accessibility, responsive design, and production asset builds.
---

# Topcoat UI

Build accessible server-rendered interfaces through the `topcoat` facade. Inspect `Cargo.toml`, `build.rs`, router asset registration, the root layout, stylesheets, and `components.toml` before changing the UI stack.

## Write views and components

Use real HTML syntax in `view!`. Quote literal text, interpolate Rust with `(expr)`, and use `if`, `for`, `match`, and `let` directly in the body.

Define reusable async `#[component]` functions. A `child: View` prop receives trailing nodes; `#[default]` makes props optional; `#[into]` converts at the call site; `cx: &Cx` is supplied implicitly.

Forward caller attributes with an `Attributes` prop and `<element (attrs)>`. Use `attributes!` to build fragments and `class!` for conditional class lists. Spreading consumes `Attributes`, and each key is unique.

Prefer semantic elements, labels, native controls, keyboard support, and visible focus. Run `topcoat fmt` after editing views.

## Bundle assets

Declare local or remote files with `asset!` and load the matching bundle on the router:

```rust
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    router::{Router, RouterBuilderDiscoverExt},
};

let router = Router::builder()
    .discover()
    .assets(AssetBundle::load().unwrap())
    .build();
```

Use `topcoat dev` during development and `topcoat asset bundle` for manual builds. Bundle with the same binary, profile, target directory, and checkout used for deployment. Add checksums to remote assets when reproducibility matters.

## Configure Tailwind

Enable `tailwind` on both the runtime dependency and a build dependency with default features disabled. In `build.rs`:

```rust
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();
}
```

Load `<link rel="stylesheet" href=(topcoat::tailwind::stylesheet!())>` in the root layout and register the asset bundle.

Topcoat uses the standalone Tailwind CLI. Keep complete utility names as literals; runtime-built names such as `format!("text-{color}-600")` are invisible to scanning. Use Tailwind source directives for precise scanning, keep `target/` ignored, and take care with Cargo `rerun-if-*` directives because the first one replaces default package-wide tracking.

## Use Topcoat UI as owned source

```sh
topcoat ui init
topcoat ui list
topcoat ui add button
```

Commit `components.toml` and `styles.css`. Installed components are ordinary application source: inspect and edit them. They commonly use enum variants, forwarded `attrs`, child content, merged classes, and `*_variants` helpers.

Theme semantic CSS variables in `styles.css`; apply `dark` to an ancestor for dark values. Re-adding with `--overwrite` replaces local component source, so diff first.

## Add fonts and icons

Use `font!` for custom faces or enable `font-fontsource` and use `fontsource_font!`. Register fonts with `.discover()` or `.font(...)`, then render `topcoat::font::link(...)` in `<head>`. Include only used weights, styles, and subsets. `host: Asset` self-hosts files and requires the asset bundle.

Use the `icon` component for inline SVG. Decorative icons omit `label`; meaningful icon-only controls need an accessible label. For Iconify, enable `icon-iconify` on runtime and build dependencies, stage sets in `build.rs`, then use `iconify::include!` or `iconify_icon!`. Pin or vendor sets for reproducible offline builds.

Verify responsive layouts, content extremes, keyboard behavior, focus, contrast, light/dark themes, and production-style font and asset URLs.
