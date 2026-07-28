# Datastar

This example demonstrates how to use Datastar with Topcoat to update browser state and HTML through Server-Sent Events.

It shows how to:

- declare a reactive signal in HTML;
- send the current signals to a Topcoat route;
- update a signal from the server;
- patch an HTML element;
- append new elements without reloading the page.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/datastar/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path datastar/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

The page loads Datastar from a CDN, so the browser needs an internet connection.

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

The counter should become:

```text
Count: 1
```

A new item should also appear below the button:

```text
Counted to 1
```

Click the button two more times.

The page should display:

```text
Count: 3
```

The log should contain:

```text
Counted to 1
Counted to 2
Counted to 3
```

The counter and log update without reloading the complete page.

## Inspect the request

Open the browser developer tools and select the **Network** panel.

Click **Increment** and inspect the request to:

```text
POST /increment
```

It should return an HTTP `200` response containing Server-Sent Events.

## Check the page route

From another terminal, run:

```sh
curl --include http://127.0.0.1:3000/
```

The response should have an HTTP `200` status and contain:

```html
<body data-signals:count="0">
```

## How it works

- `data-signals:count="0"` creates the initial browser signal.
- `data-text="$count"` displays the current signal value.
- `data-on:click="@post('/increment')"` sends an action request to the server.
- `Signals<Counter>` extracts the signal values sent by Datastar.
- `PatchSignals` sends the updated counter back to the browser.
- `PatchElements` sends an HTML fragment to the browser.
- `.selector("#log")` selects the element that should be patched.
- `ElementPatchMode::Append` adds the new entry to the existing log.
- `Sse` returns the patches as Server-Sent Events.

The counter is maintained in browser state. Reloading the page resets it to zero.

Stop the server by pressing `Ctrl+C`.