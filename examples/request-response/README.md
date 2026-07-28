# Request and response

This example demonstrates how Topcoat reads different HTTP request formats and creates different response types.

It covers:

- JSON requests and responses;
- query parameters;
- URL-encoded forms;
- raw form data;
- multipart uploads;
- optional JSON bodies;
- raw bytes and body streams;
- custom responses;
- custom request extractors.

This is an API-only example. It does not provide an HTML page at `/`.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/request-response/Cargo.toml
```

If your terminal is already inside the `examples` directory, run:

```sh
cargo run --manifest-path request-response/Cargo.toml
```

The API is served by default at:

```text
http://127.0.0.1:3000
```

## JSON request and response

Send a JSON request:

```sh
curl --include \
    --request POST \
    --header "Content-Type: application/json" \
    --data '{"name":"Francesco"}' \
    http://127.0.0.1:3000/api/users
```

Expected response body:

```json
{"name":"Francesco"}
```

## Query parameters

Send a request containing `q` and `limit`:

```sh
curl --include \
    "http://127.0.0.1:3000/api/search?q=rust&limit=5"
```

Expected response body:

```json
{"query":"rust","limit":5}
```

When `limit` is omitted, it defaults to `10`:

```sh
curl "http://127.0.0.1:3000/api/search?q=rust"
```

## URL-encoded form

Send a form body:

```sh
curl --include \
    --request POST \
    --header "Content-Type: application/x-www-form-urlencoded" \
    --data "q=topcoat&limit=8" \
    http://127.0.0.1:3000/api/form-echo
```

The response contains the parsed form values.

## Raw form data

Send an URL-encoded body without deserializing it into a Rust structure:

```sh
curl --include \
    --request POST \
    --header "Content-Type: application/x-www-form-urlencoded" \
    --data "q=rust&limit=5" \
    http://127.0.0.1:3000/api/raw-form
```

Expected response:

```text
received 14 bytes of form data
```

## Multipart form data

Create a small test file:

```sh
printf "abc" > /tmp/topcoat-demo.txt
```

Upload it together with a text field:

```sh
curl --include \
    --form "title=hello" \
    --form "file=@/tmp/topcoat-demo.txt" \
    http://127.0.0.1:3000/api/files
```

Expected response:

```text
received 8 bytes across all fields
```

The total contains five bytes from `hello` and three bytes from the uploaded file.

## Optional JSON body

Send a request without a body:

```sh
curl --request POST \
    http://127.0.0.1:3000/api/maybe-user
```

Expected response:

```text
no user provided
```

Send the same request with a JSON body:

```sh
curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"name":"Francesco"}' \
    http://127.0.0.1:3000/api/maybe-user
```

Expected response:

```text
got user Francesco
```

## Raw bytes

Send a raw request body:

```sh
curl --request POST \
    --data-binary "hello" \
    http://127.0.0.1:3000/api/bytes
```

Expected response:

```text
received 5 bytes
```

## Body stream

Send a body that is read as a stream:

```sh
curl --request POST \
    --data-binary "hello topcoat" \
    http://127.0.0.1:3000/api/upload
```

Expected response:

```text
received 13 bytes
```

## Custom CSV response

Request the CSV endpoint:

```sh
curl --include \
    http://127.0.0.1:3000/api/report.csv
```

The response includes:

```text
content-type: text/csv; charset=utf-8
```

Expected body:

```csv
name,total
Ada,42
Grace,64
```

## Custom request extractor

Send a request with the expected signature:

```sh
curl --include \
    --request POST \
    --header "Content-Type: application/json" \
    --header "X-Signature: topcoat-demo" \
    --data '{"name":"Francesco"}' \
    http://127.0.0.1:3000/api/signed
```

Expected response body:

```json
{"name":"Francesco"}
```

Sending the request without the signature:

```sh
curl --include \
    --request POST \
    --header "Content-Type: application/json" \
    --data '{"name":"Francesco"}' \
    http://127.0.0.1:3000/api/signed
```

returns an error containing:

```text
missing x-signature header
```

An incorrect signature returns:

```text
invalid x-signature header
```

## How it works

- `Json<T>` deserializes JSON requests and serializes JSON responses.
- `Form<T>` parses query strings and URL-encoded form bodies.
- `RawForm` provides access to the original form bytes.
- `Multipart` reads multipart fields and uploaded files.
- `Option<Json<T>>` accepts an optional JSON body.
- `Bytes` buffers the complete request body.
- `Body` provides access to the body stream.
- `IntoResponse` allows custom types to control the HTTP response.
- `FromRequest` allows custom types to validate and extract request data.

Stop the server by pressing `Ctrl+C`.