# App context

This example demonstrates how to share application state across HTTP requests.

It registers an in-memory page view counter as application context and increments it whenever the root page is requested.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/app-context/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

Open the address in your browser.

## Expected result

The page displays the number of requests received by the root route:

```text
This page has been viewed 1 times.
```

Refresh the page and the number should increase.

The counter is stored in memory and resets when the application is restarted.

## Test the example

With the application running, send three requests from another terminal:

```sh
for _ in 1 2 3; do
    curl --silent http://127.0.0.1:3000/ \
        | grep -oE 'viewed [0-9]+ times'
done
```

The values should increase by one after every request:

```text
viewed 1 times
viewed 2 times
viewed 3 times
```

The initial number may be higher if the page has already been opened in a browser.

You can verify the HTTP status with:

```sh
curl \
    --silent \
    --output /dev/null \
    --write-out "%{http_code}\n" \
    http://127.0.0.1:3000/
```

The expected output is:

```text
200
```

## How it works

- `PageViews` contains an atomic counter shared across requests.
- `.app_context(...)` registers the value when the router is created.
- The page handler receives the request context as `cx: &Cx`.
- `app_context::<PageViews>(cx)` retrieves the registered value by type.
- `fetch_add` increments the counter atomically.
- `#[page("/")]` registers the page at the root route.

Stop the server by pressing `Ctrl+C`.