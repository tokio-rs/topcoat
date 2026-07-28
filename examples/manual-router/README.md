# Manual router

This example demonstrates how to register Topcoat layouts, pages, and routes manually.

It shows how to:

- build a router without automatic discovery;
- register layouts and pages explicitly;
- use nested layouts;
- create a plain-text API route.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/manual-router/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Available pages

Open the following pages in your browser:

| Path | Expected content |
| --- | --- |
| `/` | The home page |
| `/about` | The about page |
| `/docs` | The docs page inside the docs layout |
| `/docs/install` | The install page inside the docs layout |
| `/api/health` | The plain-text response `ok` |

All HTML pages are wrapped by the root layout.

The `/docs` and `/docs/install` pages are additionally wrapped by the nested docs layout.

## Test the example

Check every route from another terminal:

```sh
for path in / /about /docs /docs/install /api/health; do
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

Test the API route directly:

```sh
curl http://127.0.0.1:3000/api/health
```

The expected response is:

```text
ok
```

## How it works

- `router()` creates the router and registers every item explicitly.
- `.layout(...)` registers a layout.
- `.page(...)` registers an HTML page.
- `.route(...)` registers a non-page HTTP route.
- `root_layout` wraps every page.
- `docs_layout` wraps pages whose paths begin with `/docs`.
- `slot` contains the page or nested layout rendered inside a layout.
- `health` returns a plain-text API response.

This example does not use `RouterBuilderDiscoverExt::discover()`. Every layout, page, and route must therefore be added manually to `router()`.

Stop the server by pressing `Ctrl+C`.