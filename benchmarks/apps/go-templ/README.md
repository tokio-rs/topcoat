# Harbor Market: Go + templ

The Harbor Market storefront from [`SPEC.md`](../../SPEC.md) written as a standard-library Go web app with [templ](https://templ.guide) components. The markup is a one-to-one port of the reference app in [`apps/axum-maud`](../axum-maud).

## Stack

| Piece | Version |
|-------|---------|
| Go, `net/http` server and the pattern-matching `ServeMux` (method and wildcard patterns) | Go 1.26.8 |
| [templ](https://github.com/a-h/templ) components, CLI and runtime module | v0.3.1020 |
| `encoding/json` for the dataset and the API | standard library |
| [`klauspost/compress/gzhttp`](https://github.com/klauspost/compress/tree/master/gzhttp) response compression | v1.20.4 |
| Tailwind CSS standalone CLI, run by `go generate` | v4 (built with 4.3.3) |
| Client-side behavior | `assets/app.js`, plain JS, the reference script unchanged (about 1.9 KB) |

No router framework and no other dependencies.

## Building and running

```sh
go generate ./...                                    # templ generate + tailwindcss
go build -trimpath -ldflags '-s -w' -o bin/harbor-go-templ .
PORT=8080 BENCH_THREADS=4 BENCH_COMPRESSION=on bin/harbor-go-templ   # data from ../../data by default
```

`go generate` needs `templ` and `tailwindcss` on `PATH`; the repository's `nix develop .#benchmarks` shell provides both at the versions above. The runner builds with `GOTOOLCHAIN=local` (use the shell's Go, never download one) and `CGO_ENABLED=0` (a static binary, as for a typical container deployment). The stripped binary is about 6.7 MiB with the stylesheet and script embedded.

## Layout

- `main.go`: configuration from the environment, server setup and graceful shutdown. `routes.go`: the `ServeMux` routes. `pages.go`, `api.go`: the HTML and JSON handlers, methods on an `app` that holds the store. `cart.go`: the `cart_count` cookie.
- `internal/catalog`: the dataset types and the `Store`, which loads the JSON files once at startup and builds every index (product by id, category by slug, per-category and global sort orders, featured list, related products, orders newest first, order totals). Requests only index and slice.
- `internal/views`: the templ components, shared views and one file per page, plus `views.go` with the class tables, URL builders and small helpers.
- `internal/format`: money, count, rating and date formatting.
- `assets`: the Tailwind input, the script and the `embed`-based asset handler.

## Mapping the benchmark settings

- `BENCH_THREADS`: unset or `0` keeps Go's default `GOMAXPROCS`, one P per core. `N` calls `runtime.GOMAXPROCS(N)`, so at most N threads run Go code at once, including the garbage collector's workers. `GOGC` is left at its default.
- `BENCH_COMPRESSION=on` wraps the mux in `gzhttp.GzipHandler` with its defaults: gzip and zstd (zstd preferred when the client accepts both), default levels, responses under 1024 bytes left uncompressed. Go's standard library has no compression middleware; gzhttp is the most widely used one (it started as a fork of NYTimes' `gziphandler` and is maintained with the `klauspost/compress` encoders).

## Judgment calls

- **Generated templ code is committed.** The `*_templ.go` files sit next to their `.templ` sources, following templ's guidance that ["it's common practice to commit generated `*_templ.go` files"](https://github.com/a-h/templ/blob/main/docs/docs/09-developer-tools/06-cicd.md) so that the code builds with plain `go build`. The build still runs `templ generate` (through `go generate`), so the benchmark always uses output from the shell's templ CLI, and `git diff` after a build shows whether the committed files were stale. The compiled stylesheet in `assets/dist/` is build output and is not committed.
- **Rendering goes through `templ.Handler`** with `templ.WithStatus` for the 404 and 422 pages. That is templ's documented default: it [renders into a pooled buffer and sends it once the template has rendered](https://templ.guide/server-side-rendering/streaming), so a failed render gives a clean error instead of half a page. Streaming (`templ.WithStreaming()`) only pays off when a page waits on slow data, which this app never does. `templ.Handler` also sets `Content-Type: text/html; charset=utf-8`.
- **Components follow templ's usual style.** Pages take the store or the product and the cart count as typed parameters, the layout wraps page content with `{ children... }`, and data-dependent class strings come from `switch` tables in `views.go` and are concatenated in `class={ "fixed part " + table(value) }`, so Tailwind sees every literal. The Tailwind source scan covers `internal/`, which includes the `.templ` and `.go` files. Dynamic `class` and `href` values go through templ's normal class handling and URL sanitization; nothing bypasses templ's escaping. templ escapes `'` and `"` as `&#39;` and `&#34;` (Go's `html.EscapeString`), where Maud writes `'` and `&quot;`; the rendered text is identical.
- **Routing uses the Go 1.22+ `ServeMux` patterns** (`GET /products/{id}`, `GET /{$}`, `POST /api/cart`) with a catch-all `/` handler that renders the HTML 404 page, including for unknown `/api` paths. A request with a known path but the wrong method falls through to the catch-all, so it gets the 404 page rather than the mux's 405.
- **Query strings are parsed with `url.ParseQuery`**, so a malformed query (a bad `%` escape) is the 422 "invalid query" case from the spec instead of being silently dropped as `r.URL.Query()` would. Repeated parameters use the first value, as `url.Values.Get` does.
- **The cart API** checks for an `application/json` Content-Type, reads at most 1 MiB with `http.MaxBytesReader`, decodes with `json.Decoder` into pointer fields so that missing and `null` values are rejected, and rejects trailing data after the object. Responses are written with `json.NewEncoder(w).Encode`, which escapes `<`, `>` and `&` in strings as `<` and so on by default; the parsed values are identical.
- **The listing API reuses the product struct.** `Product` embeds a `ProductSummary` with the 13 fields of a list item, so the list endpoint encodes `&product.ProductSummary` directly, and the detail endpoint encodes the whole `Product`, which `encoding/json` flattens back into the shape of `products.json`.
- **Dates are parsed at startup** into `time.Time` (a `catalog.Date` type that marshals as `2006-01-02`), like typed database columns, and formatted per request with Go's layout strings (`"Jan 2, 2006"`, `"Jan 2, 2006 15:04"`). Money and counts are formatted per request by small functions in `internal/format`; the standard library has no number grouping, and `golang.org/x/text/message` would be a heavier dependency for two lines of arithmetic.
- **Assets** are embedded with `embed` and served under URLs carrying a 64-bit FNV-1a hash of their content (computed at startup with `hash/fnv`), with `Cache-Control: public, max-age=31536000, immutable`, through `http.ServeContent` (which also handles `HEAD`, ranges and conditional requests).
- **Server settings**: `ReadHeaderTimeout` (10 s) and `IdleTimeout` (2 min) are set, as Go production guidance recommends for an internet-facing `http.Server`; neither limits a busy keep-alive connection. `SIGTERM` triggers `Server.Shutdown`. `net/http` writes no access log; errors go to stderr through the `log` package.

## What a critic could point at

- templ's runtime does some work per expression that a hand-written renderer would not: every `{ expr }` goes through `fmt.Sprint`, every dynamic `class={ ... }` builds a small de-duplicating class list, and every component call derives a context. This is the cost of using templ as documented, and it shows most on the 500-row orders page. Nothing here works around it.
- `encoding/json` (v1) is reflection-based. The spec asks for the ecosystem's standard JSON support, so faster third-party encoders are not used.
