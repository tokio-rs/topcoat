# htmx

This example demonstrates how to use htmx with Topcoat to update part of a server-rendered page.

It shows how to:

- detect requests sent by htmx;
- return an HTML fragment instead of the complete document;
- replace a targeted element without reloading the page;
- share application state across requests;
- send a custom htmx event through a response header.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/htmx/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path htmx/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

The page loads htmx from a CDN, so the browser needs an internet connection.

## Test in the browser

Open:

```text
http://127.0.0.1:3000
```

The page should initially display:

```text
Count: 0
```

Click **Increment**.

The value should become:

```text
Count: 1
```

Continue clicking the button. The value should increase without reloading the complete page.

## Test the htmx response

With the server running, send an htmx request from another terminal:

```sh
curl --include \
    --request POST \
    --header "HX-Request: true" \
    http://127.0.0.1:3000/increment
```

The response should have an HTTP `200` status and contain an HTML fragment similar to:

```html
<span id="count">1</span>
```

The exact number may be higher if the counter has already been incremented.

The response should also include:

```text
hx-trigger: counted
```

## Test fragment rendering

Send a normal request to the root route:

```sh
curl --silent http://127.0.0.1:3000/
```

The response should contain the complete HTML document.

Send the same request as htmx:

```sh
curl --silent \
    --header "HX-Request: true" \
    http://127.0.0.1:3000/
```

This response should contain the page content without the outer HTML document.

## How it works

- `hx_request(cx)` detects the `HX-Request` request header.
- Regular requests receive the complete HTML shell.
- htmx requests receive only the requested page fragment.
- `hx-post="/increment"` sends a `POST` request when the button is clicked.
- `hx-target="#count"` selects the element to update.
- `hx-swap="innerHTML"` replaces the contents of the selected element.
- `Counter` stores the shared value in an atomic integer.
- `app_context::<Counter>(cx)` retrieves the counter.
- `HxResponseTrigger::receive(["counted"])` adds an `HX-Trigger` response header.

The counter is stored only in memory and resets when the server restarts.

Stop the server by pressing `Ctrl+C`.