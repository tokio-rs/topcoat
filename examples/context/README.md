# Context

This example demonstrates how to use Topcoat's request context, `Cx`, to access information about the current HTTP request.

It shows how to:

- receive `Cx` in a page handler;
- read the current request path;
- read an HTTP request header;
- use request data while rendering a response.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/context/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

Open the address in your browser.

## Expected result

The page displays:

```text
Cx functions
path: /
user agent: <your browser's user agent>
```

The exact user-agent value depends on the browser or HTTP client making the request.

## Test the example

With the application running, send a request with a custom `User-Agent` header:

```sh
curl \
    --header "User-Agent: topcoat-context-test" \
    http://127.0.0.1:3000/
```

The response should contain:

```text
path: /
user agent: topcoat-context-test
```

You can also verify the HTTP status:

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

- `Cx` provides access to information associated with the current request.
- The page handler receives it as `cx: &Cx`.
- `uri(cx).path()` returns the requested path.
- `headers(cx)` returns the request headers.
- `.get("user-agent")` reads the optional `User-Agent` header.
- `"unknown"` is used when the header is missing or cannot be converted to text.
- `#[page("/")]` registers the page at the root route.

Stop the server by pressing `Ctrl+C`.