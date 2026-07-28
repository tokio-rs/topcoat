# Module router

This example demonstrates how Topcoat can derive routes from the Rust module tree.

Instead of registering every page, layout, and route manually, the application builds its router with:

```rust
topcoat::router::module_router!().build()
```

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/module-router/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Available routes

| Source | URL | Expected result |
| --- | --- | --- |
| `src/app.rs` | `/` | Home page |
| `src/app.rs`, module `about` | `/about` | About page |
| `src/app/docs.rs` | `/docs` | Docs page inside the docs layout |
| `src/app/docs/install.rs` | `/docs/install` | Install page inside the docs layout |
| `src/app/_marketing/pricing.rs` | `/pricing` | Pricing page inside the marketing layout |
| `src/app/api/health.rs` | `/api/health` | Plain-text response `ok` |

## Test the example

Open these pages in your browser:

```text
http://127.0.0.1:3000/
http://127.0.0.1:3000/about
http://127.0.0.1:3000/docs
http://127.0.0.1:3000/docs/install
http://127.0.0.1:3000/pricing
```

Check all route status codes from another terminal:

```sh
for path in / /about /docs /docs/install /pricing /api/health; do
    status=$(curl --silent \
        --output /dev/null \
        --write-out "%{http_code}" \
        "http://127.0.0.1:3000$path")

    echo "$path -> $status"
done
```

Every route should return:

```text
200
```

Test the API route:

```sh
curl http://127.0.0.1:3000/api/health
```

The expected response is:

```text
ok
```

Verify the nested docs layout:

```sh
curl --silent http://127.0.0.1:3000/docs/install \
    | grep -E "docs layout|install"
```

The response should contain both:

```text
docs layout
install
```

Verify the marketing group layout:

```sh
curl --silent http://127.0.0.1:3000/pricing \
    | grep -E "marketing group layout|pricing"
```

The response should contain both:

```text
marketing group layout
pricing
```

## How it works

- `main.rs` starts the router returned by `app::router()`.
- `module_router!()` derives routes from the module structure under `app`.
- The root `app` module corresponds to `/`.
- Child module names become URL segments.
- `app::docs` corresponds to `/docs`.
- `app::docs::install` corresponds to `/docs/install`.
- Modules beginning with an underscore group routes without adding a URL segment.
- `_marketing::pricing` therefore corresponds to `/pricing`, not `/_marketing/pricing`.
- Layouts wrap the pages defined inside their module hierarchy.
- `app::api::health` becomes the `GET /api/health` route.

Stop the server by pressing `Ctrl+C`.