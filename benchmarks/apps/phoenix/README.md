# Harbor Market: Phoenix

The Harbor Market storefront as a standard Phoenix application: controllers render HEEx templates built from function components, the JSON API goes through an `:api` pipeline with JSON views, and the app ships as a `mix release` served by Bandit. See [`SPEC.md`](../../SPEC.md) for the contract.

## Stack

| Piece | Version |
|-------|---------|
| Elixir / Erlang/OTP (from the Nix shell) | 1.18.5 / OTP 28 (ERTS 16.4) |
| [Phoenix](https://www.phoenixframework.org), generated with `phx.new` 1.8.16 | 1.8.16 |
| Phoenix LiveView (for HEEx and `Phoenix.Component` only, no LiveViews) | 1.2.13 |
| [Bandit](https://github.com/mtrudel/bandit) HTTP server (Phoenix default) | 1.14.1 |
| Jason (the generated JSON library) | 1.4.5 |
| `tailwind` hex package, Tailwind CSS CLI | 0.5.2, 4.3.3 (the shell's version) |
| `esbuild` hex package, esbuild | 0.10.1, 0.25.4 |
| Client-side behavior | `assets/js/app.js`, plain JS, 1.2 KB minified |

Exact dependency versions are in `mix.lock`.

## Running

From the repository root, in the Nix shell (`nix develop .#benchmarks`), the runner builds and starts the app from `app.toml`. By hand:

```sh
cd benchmarks/apps/phoenix
export MIX_ENV=prod MIX_HOME=.mix HEX_HOME=.hex MIX_REBAR3=rebar3
mix local.hex --force --if-missing
mix deps.get --only prod && mix compile && mix assets.deploy && mix release --overwrite
PORT=4000 BENCH_DATA_DIR=$PWD/../../data PHX_SERVER=true SECRET_KEY_BASE=... \
  _build/prod/rel/harbor/bin/harbor start
```

For development, `mix setup` and `mix phx.server` work as in any Phoenix app (data defaults to `../../data`).

`MIX_HOME` and `HEX_HOME` keep Hex and its package cache inside the app directory, and `MIX_REBAR3=rebar3` compiles the rebar3 dependencies (telemetry) with the shell's `rebar3` rather than a copy installed into `MIX_HOME`. The tailwind and esbuild packages download their pinned binaries into `_build` on first use, as in every Phoenix app.

## Layout

- `lib/harbor/catalog.ex`, `lib/harbor/sales.ex`: the contexts. They load the JSON files once at startup, build the indexes and answer the queries (product by id, category by slug, sorted listing pages, featured and related products, orders newest first, order totals).
- `lib/harbor_web/router.ex`: the generated `:browser` and `:api` pipelines, plus the `CartCount` plug that assigns the cart badge count from the cookie.
- `lib/harbor_web/controllers/`: one controller and one `*HTML` or `*JSON` view module per resource, templates in `*_html/`. `FallbackController` turns `{:error, reason}` into the HTML error page or a JSON error.
- `lib/harbor_web/components/`: the root layout and `Layouts.app` (header, footer), `StoreComponents` (product card and tile, star row, breadcrumbs, pagination, status badge, color palette, listing links) and `Icons`.
- `lib/harbor_web/format.ex`: money, count, rating and date formatting, imported into every template.
- `rel/env.sh.eex`: the release environment hook that maps `BENCH_THREADS` to scheduler flags.

## Benchmark settings

| Setting | How it maps |
|---------|-------------|
| Production mode | `MIX_ENV=prod`, run from a [`mix release`](https://hexdocs.pm/phoenix/releases.html) (`bin/harbor start`, `PHX_SERVER=true`), with digested assets from `mix assets.deploy` served by `Plug.Static` under `?vsn=d` URLs with `Cache-Control: public, max-age=31536000, immutable`. Templates are compiled into the release, protocols consolidated. |
| `PORT` | `config/runtime.exs`, bound to `127.0.0.1`. |
| `BENCH_THREADS` | Unset or `0`: the BEAM default, one scheduler per core. `N`: `rel/env.sh.eex` exports `ELIXIR_ERL_OPTIONS="+S N:N +SDcpu N:N"`, so N normal schedulers and N dirty CPU schedulers are online ([erl flags](https://www.erlang.org/doc/apps/erts/erl_cmd.html)). The release hook is the place the [release docs](https://hexdocs.pm/mix/Mix.Tasks.Release.html) give for VM flags that depend on the environment. |
| `BENCH_COMPRESSION` | Bandit compresses responses (zstd, gzip, deflate) by default when the client sends `Accept-Encoding`. `runtime.exs` sets Bandit's `http_options: [compress: true \| false]` from the variable ([Bandit options](https://hexdocs.pm/bandit/Bandit.html#t:http_options/0)), so `off` really sends identity responses. |
| Logging | `config :logger, level: :warning` in `prod.exs`: no per-request log lines; warnings and errors still log. |
| `SECRET_KEY_BASE` | A fixed dummy value in `app.toml`. It signs the (unused) session cookie and nothing else in this app. |
| Distribution | `RELEASE_DISTRIBUTION=none` in `rel/env.sh.eex`: no node name, no `epmd` daemon outliving the server's process group. |

## Judgment calls

- **Controllers and HEEx, not LiveView.** Every page is a regular controller action rendering a HEEx template, the standard choice for request/response pages ([Phoenix components and HEEx](https://hexdocs.pm/phoenix/components.html)). The load generator only fetches documents, so a LiveView's websocket would never be opened. The app was generated with `--no-live`; `phoenix_live_view` stays as a dependency because it provides HEEx and `Phoenix.Component`.
- **Client JS.** `app.js` holds only the two interactive pieces as plain JS, bundled by the standard esbuild setup. See the removal list below for what was taken out of the generated `app.js` and root layout.
- **Removed from the generated app (SPEC rule 12).** Each piece either serves a feature the app does not have or ships behavior SPEC excludes:
  - daisyUI, the heroicons dependency and `core_components.ex` (flash, forms and tables in daisyUI classes), the theme-toggle script and the topbar vendor file. SPEC requires plain Tailwind with the default theme and inline SVG icons; `assets/css/app.css` is `@import "tailwindcss"` plus the source paths.
  - The LiveView socket setup, through `phx.new --no-live`, because there are no LiveViews.
  - `import "phoenix_html"` and the flash-close handler in `app.js`. `phoenix_html.js` only implements `data-method`/`data-confirm` links, and the app has none of those and no flash messages. SPEC allows no client behavior beyond the two interactive pieces.
  - `<meta name="csrf-token">` in the root layout. It exists for the LiveView socket and `phoenix_html` links. Calling `get_csrf_token()` would also store a token in the session, so every HTML response to a client without a session cookie (every benchmark request) would carry a `Set-Cookie: _harbor_key=...`. As shipped, no response sets a cookie.
  - `favicon.ico` and `images/logo.svg`. SPEC uses `<link rel="icon" href="data:,">` and inline SVG.
  - Ecto, Swoosh, LiveDashboard and Gettext, through generator flags: no database, no mail, no dashboard, English-only copy.
- **Changed from the generated config.** Logger level is `:warning` instead of `:info`, so there are no per-request logs (SPEC rule 6). The endpoint binds 127.0.0.1 instead of all interfaces. `url` is `http://localhost`. Bandit's `compress` follows `BENCH_COMPRESSION`. `check_origin` is left as generated; it only applies to sockets, which the app does not have.
- **Kept as generated.** The endpoint plugs (`Plug.Static`, `Plug.RequestId`, `Plug.Telemetry`, `Plug.Parsers`, `Plug.MethodOverride`, `Plug.Head`, `Plug.Session`), the full `:browser` pipeline (`fetch_session`, `fetch_live_flash`, `protect_from_forgery`, `put_secure_browser_headers`, which adds the `content-security-policy`, `referrer-policy`, `x-content-type-options` and `x-permitted-cross-domain-policies` headers), `force_ssl` in `prod.exs` (it excludes `localhost` and `127.0.0.1` as generated, so it passes loopback requests through), the telemetry supervisor, PubSub and DNSCluster. These are what a Phoenix storefront runs in production, so their per-request cost is part of the measurement.
- **Data in `:persistent_term`.** `Harbor.Catalog.load!/1` and `Harbor.Sales.load!/1` run in `Application.start/2` before the endpoint starts. Products sit in a tuple indexed by id, categories in a map by slug, each `{category, sort}` listing order is a precomputed tuple of ids, and the order list and its totals are computed once. [`:persistent_term`](https://www.erlang.org/doc/apps/erts/persistent_term.html) is built for read-only data that is read constantly and written once: reads take no lock and never copy the term into the request process, unlike ETS. Listing pages index into the id tuple, so every per-request lookup is O(1) per item.
- **Formatting.** Dates go through `Calendar.strftime/2` (`"%b %-d, %Y"`, `"%b %-d, %Y %H:%M"`, and `"%Y-%m-%d"` / `"%Y-%m-%dT%H:%MZ"` for `datetime` attributes) on the `Date` and `DateTime` values parsed at load. Elixir's standard library has no thousands-separator or currency formatter, so `money/1` and `count/1` are small hand-written functions. Ratings use `:erlang.float_to_binary(r, decimals: 1)`; whole ratings are stored as integers in the JSON file and converted to floats at load. Filled stars are `div(tenths + 5, 10)`.
- **Errors.** Controllers return `{:error, reason}` and `action_fallback` renders the error ([`action_fallback/1`](https://hexdocs.pm/phoenix/Phoenix.Controller.html#action_fallback/1)), picking `ErrorHTML` or `ErrorJSON` by the request format. Unknown routes go through Phoenix's `render_errors`, configured with the root layout so the 404 page has the full layout too; there the cart count is read from the request because the router pipeline never ran.
- **Malformed JSON is a 422.** Plug answers an unparsable body with a 400. SPEC wants every invalid cart request to be a 422 with `{"error": "invalid request body"}`, so the endpoint wraps `Plug.Parsers` in a small `parse_body` plug that rescues `Plug.Parsers.ParseError`. Everything else about body parsing is the generated configuration.
- **Links** are all built with verified routes (`~p`), including the static nav and footer links (`~p"/about#shipping"`). Listing links pass a query keyword list (`~p"/products?#{query}"`) that leaves out default values.

## Things a critic could point at

- The BEAM keeps its default scheduler busy-waiting (`+sbwt`), which spends some CPU spinning when a scheduler runs out of work. Under full load this hardly matters, but it lowers requests per CPU-second a little at partial load. It is left at the default because that is what Phoenix deployments run.
- On a busy 8-core M3 (4 performance and 4 efficiency cores) the BEAM did not go much above 3 cores in the noisy development runs, even with spare CPU and an idle load generator. One candidate is the VM's single default I/O poll thread (`+IOt 1`) on macOS kqueue: one sample with `+IOt 4 +IOp 4` nearly doubled throughput, but repeated runs were drowned out by other builds. A later `standard` run on a quiet machine did not reproduce the cap (6.6 to 7.0 of 8 cores on the HTML scenarios), so the defaults are kept.
- The release includes ERTS (about 6 MB of the 24 MB release) so it is self-contained.
- Idle memory includes the BEAM's preallocated memory carriers and all loaded code, so it starts around 170 MB RSS on macOS.
