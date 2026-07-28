# Cookie

This example demonstrates how to store typed application data in a signed cookie.

It uses a visit counter that is read, incremented, and written back whenever the root page is requested.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/cookie/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path cookie/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Test in the browser

Open:

```text
http://127.0.0.1:3000
```

The first request should display:

```text
You have visited this page 1 times.
```

Refresh the page. The number should increase:

```text
You have visited this page 2 times.
```

Refresh it again:

```text
You have visited this page 3 times.
```

The value is stored in the browser cookie rather than in server-side application state.

## Test with curl

Remove any cookie jar left by an earlier test:

```sh
rm -f /tmp/topcoat-cookie.jar
```

Send three requests while storing and reusing cookies:

```sh
for _ in 1 2 3; do
    curl --silent \
        --cookie /tmp/topcoat-cookie.jar \
        --cookie-jar /tmp/topcoat-cookie.jar \
        http://127.0.0.1:3000/ \
        | grep -oE 'visited this page [0-9]+ times'
done
```

Expected output:

```text
visited this page 1 times
visited this page 2 times
visited this page 3 times
```

Inspect the response headers:

```sh
curl --include \
    --cookie /tmp/topcoat-cookie.jar \
    --cookie-jar /tmp/topcoat-cookie.jar \
    http://127.0.0.1:3000/
```

The response should contain a `Set-Cookie` header for the `visits` cookie.

## How it works

- `.cookies()` enables cookie handling on the router.
- `Key::generate()` creates the key used to sign and verify cookies.
- `signed_cookies(cx)` returns a signed cookie jar.
- `default_path("/")` makes the cookie available across the application.
- `default_http_only(true)` prevents browser JavaScript from reading the cookie.
- `override_secure(true)` marks the cookie as secure.
- `CookieStore<Visits, _>` serializes and deserializes the typed counter.
- `parse_or_default()` starts from zero when the cookie is missing or invalid.
- `update(Visits::increment)` increments the stored value.
- `commit()` queues the updated `Set-Cookie` response header.

Signing protects the cookie from modification but does not encrypt its value.

This example generates a new signing key whenever the server starts. Restarting the application invalidates the previous cookie and resets the counter. A real application should load a persistent secret key.

Stop the server by pressing `Ctrl+C`.