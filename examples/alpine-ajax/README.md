# Alpine AJAX

This example demonstrates how to use Alpine AJAX with Topcoat to update part of a server-rendered page.

It shows how to:

- detect requests sent by Alpine AJAX;
- return only the HTML element requested by the client;
- replace a targeted element without reloading the complete page;
- preserve a normal HTML form fallback when JavaScript is unavailable;
- share a counter across requests with application context.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/alpine-ajax/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path alpine-ajax/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

The page loads Alpine.js and Alpine AJAX from a CDN, so the browser needs an internet connection.

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

The count should become:

```text
Count: 1
```

Continue clicking the button. The number should increase without reloading the complete page.

The form targets the element with:

```html
<span id="count">
```

through:

```html
x-target="count"
```

## Test the Alpine AJAX response

Send a request containing the Alpine AJAX headers:

```sh
curl --include \
    --request POST \
    --header "X-Alpine-Request: true" \
    --header "X-Alpine-Target: count" \
    http://127.0.0.1:3000/increment
```

The response should have an HTTP `200` status and contain only the targeted element:

```html
<span id="count">1</span>
```

The exact number may be higher if the counter was already incremented.

## Test the non-JavaScript fallback

Send the same request without Alpine AJAX headers:

```sh
curl --include \
    --request POST \
    http://127.0.0.1:3000/increment
```

The response should redirect to:

```text
/
```

Follow the redirect:

```sh
curl --location \
    --request POST \
    http://127.0.0.1:3000/increment
```

The returned full page should display the updated count.

## How it works

- `Counter` stores the shared value in an atomic integer.
- `.app_context(...)` registers the counter for the complete application.
- `app_context::<Counter>(cx)` retrieves it from the request context.
- `ajax_request(cx)` checks for `X-Alpine-Request: true`.
- `x-target="count"` tells Alpine AJAX which element should be replaced.
- An Alpine AJAX request receives only the updated `<span>`.
- A regular form submission receives a redirect to `/`.
- The regular redirect provides a working fallback without JavaScript.

The counter is stored only in memory and resets when the server restarts.

Stop the server by pressing `Ctrl+C`.