# Harbor Market: Leptos islands

The Harbor Market storefront in Leptos's [islands mode](https://book.leptos.dev/islands.html): every component renders on the server only, and the two interactive pieces are `#[island]`s that a small WebAssembly bundle hydrates. It is the [`leptos`](../leptos) app with the changes islands call for and nothing else, so comparing the two isolates islands against whole-app hydration. Both are standalone apps on purpose.

## Stack

| Piece | Version |
|-------|---------|
| Leptos with the `islands` feature, `leptos_router`, `leptos_meta` | 0.8.23, 0.8.17, 0.8.7 |
| `leptos_axum` on Axum and Tokio, HTTP/1.1 | 0.8.10, axum 0.8.9, tokio 1.54.1 |
| tower-http `CompressionLayer` | 0.7.1 |
| `cargo-leptos` | 0.3.10 (from the Nix shell) |
| `wasm-bindgen` (crate pinned to match the CLI), `wasm-opt` | 0.2.129, binaryen 132 |
| Tailwind CSS, run by cargo-leptos from `PATH` | 4.3.3 |

## Running

```sh
cargo leptos build --release
LEPTOS_OUTPUT_NAME=harbor-leptos-islands LEPTOS_SITE_ROOT=target/site LEPTOS_ENV=PROD \
  PORT=3000 target/benchmark/harbor-leptos-islands
```

Build profiles, features, threads, compression, logging, template pieces left out, Tailwind and static assets work exactly as in the [`leptos` README](../leptos/README.md#build-and-settings), and so do date formatting (`time`'s `format_description!`), links (plain `<a href>` from small URL builders, since Leptos has no URL generation from routes) and the JSON API (plain Axum handlers); see its [judgment calls](../leptos/README.md#judgment-calls).

## What differs from the `leptos` app

- **Islands.** `leptos` has the `islands` feature, the shell uses `<HydrationScripts options islands=true/>` and the WebAssembly entry point calls `hydrate_islands()`, as in the [islands guide](https://book.leptos.dev/islands.html). `MenuToggle` (in `components/header.rs`) and `AddToCart` (in `pages/detail.rs`) are `#[island]`s; their bodies are the same signal code as the components in the `leptos` app. `AddToCart` takes `max` as a prop, which Leptos serializes into the island's `data-props` attribute.
- **Data access.** Components that are not islands never run in the browser, so pages read the store from context and render synchronously. The guide calls out this removal of `#[server]` functions, resources and `<Suspense>` as a benefit of islands. Each page keeps the same `*Data` struct and loading code as the `leptos` app, only called directly instead of through a server function, and nothing is serialized for hydration. Without async data the pages render with the default SSR mode; no route needs `SsrMode::Async`.
- **State outside the islands.** The navigation and the cart badge are server-rendered HTML outside any island, and the two islands are not nested, so they cannot share a signal through context. `MenuToggle` toggles the `hidden` class of `#site-nav` through the DOM, and `AddToCart` writes the new count into the `[data-cart-count]` badge, the same elements the reference's script updates. The header reads the cart cookie on the server with axum-extra's `CookieJar`; `AddToCart` reads it in the browser by splitting the `document.cookie` string.
- **Island wrappers and layout.** Each island renders inside a `<leptos-island>` element. In the header that element would be an extra flex item next to the cart pill, which shifts the pill by the 8px gap on desktop even when the button is hidden. The stylesheet input therefore has one rule besides Tailwind, `leptos-island { display: contents; }`, so the islands lay out exactly like the reference (checked in headless Chrome).
- **Routing.** Islands mode has no client-side router (the optional islands router is not enabled), so links are ordinary page loads and the footer's API link needs no `rel="external"`.
- **Recursion limit.** With the pages rendering synchronously, their statically typed views nest deeply enough that the release build needs `#![recursion_limit = "256"]` in `main.rs`.

## What islands save

HTML per page with `cart_count=3`, raw / gzip -9 bytes:

| Page | Reference | Leptos | Leptos islands |
|------|-----------|--------|----------------|
| `/about` | 9,019 / 2,692 | 10,099 / 3,157 | 11,381 / 3,572 |
| `/` | 30,491 / 4,653 | 35,829 / 5,810 | 32,926 / 5,557 |
| `/products?category=outdoor&sort=price-asc&page=1` | 60,638 / 5,331 | 70,675 / 7,664 | 63,241 / 6,280 |
| `/products?page=3` | 60,422 / 5,644 | 70,240 / 8,286 | 63,000 / 6,586 |
| `/products/61` | 30,510 / 4,906 | 35,472 / 6,082 | 33,002 / 5,845 |
| `/admin/orders` | 338,581 / 22,394 | 455,947 / 38,855 | 340,936 / 23,438 |

Islands pages carry no serialized resource data. What remains above the reference is the inline islands loader (about 1.8 KB, larger than the whole-app hydration loader), the preload links and the hydration comment markers that Leptos still writes around dynamic parts of the server-rendered views.

Client assets: `harbor-leptos-islands.wasm` 196 KB (84 KB gzip, 70 KB brotli) against 862 KB (322 KB gzip, 248 KB brotli) for whole-app hydration, `harbor-leptos-islands.js` 15 KB against 24 KB, and the same 41 KB stylesheet. The islands loader imports the bundle on idle and the runner finds it through the `<link rel="preload">`, so `extra_assets` stays empty.
