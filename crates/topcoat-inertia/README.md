# Topcoat Inertia

`topcoat-inertia` implements the [Inertia.js v3 protocol](https://inertiajs.com/docs/v3/core-concepts/the-protocol) for Topcoat. It renders ordinary document requests through an application root view and returns v3 page JSON for Inertia visits.

Applications normally enable the `inertia` feature on the facade crate and import this API from `topcoat::inertia`:

```toml
[dependencies]
topcoat = { version = "0.5.0", features = ["inertia"] }
```

Install [`InertiaConfig`] as a router layer, followed by the cookie layer required by the default encrypted flash store. The application must also register one persistent cookie [`Key`](topcoat_cookie::Key) as app context.

Build pages with [`Inertia`]. Plain, lazy, partial, deferred, merge, once, rescued, shared, nested, and infinite-scroll props all use the same composable [`Prop`] value.

```rust
use topcoat_core::{context::Cx, error::Result};
use topcoat_inertia::{Inertia, defer};

async fn stats() -> Result<u64> {
    Ok(42)
}

async fn page(cx: &Cx) -> Result<topcoat_inertia::InertiaResponse> {
    Inertia::new("Dashboard")
        .prop("greeting", "Hello")
        .prop_with("stats", defer(stats()).group("dashboard").rescue())
        .render(cx)
        .await
}
```

See the Topcoat [Inertia guide](https://docs.rs/topcoat/latest/topcoat/inertia/index.html) for router and client setup. The focused guides explain [props](docs/props.md), [flash storage](docs/flash.md), and [validation errors](docs/validation.md).
