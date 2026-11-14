# Harbor Market: ASP.NET Core

The Harbor Market storefront from [`SPEC.md`](../../SPEC.md) on ASP.NET Core: Razor Pages for the HTML, Minimal API endpoints for the JSON API, and the reference app's plain JS file for the two interactive pieces.

## Stack

| Piece | Version |
|-------|---------|
| .NET SDK / runtime (from the Nix shell) | SDK 10.0.401, runtime 10.0.12 |
| ASP.NET Core shared framework, Kestrel (HTTP/1.1, socket transport) | 10.0.12 |
| Razor Pages, compiled at build time by the Razor SDK | 10.0 |
| Minimal APIs with source-generated `System.Text.Json` | 10.0 |
| Response Compression middleware | 10.0 |
| Tailwind CSS standalone CLI, run by an MSBuild target | 4.3.3 (the shell's `tailwindcss`) |
| Client-side behavior | `wwwroot/js/app.js`, plain JS, about 1.9 KB |

Everything comes from the shared framework, so the app has no NuGet package references; `packages.lock.json` records that. `nuget.config` keeps any restored packages in `.nuget/packages` inside this directory (`NUGET_PACKAGES` only accepts absolute paths, so it cannot be set from `app.toml`).

## Running

```sh
nix develop .#benchmarks -c sh -c 'cd benchmarks/apps/aspnet && dotnet publish -c Release -o bin/publish && PORT=3000 ./run.sh'
```

`run.sh` starts `dotnet Harbor.dll` from `bin/publish`, the way the [hosting docs](https://learn.microsoft.com/aspnet/core/host-and-deploy/linux-nginx#create-the-service-file) run a published app (working directory = publish directory = content root). `BENCH_DATA_DIR` defaults to `../../data`. During development, `dotnet run` works too.

## Production settings

- **Publish**: `dotnet publish -c Release`, framework-dependent and portable (no RID), which is the default output of `dotnet publish`. No ReadyToRun: it is an opt-in startup optimization that needs a RID-specific publish and does not change steady-state throughput, since tiered compilation recompiles hot code anyway. No Native AOT: [Razor Pages and MVC do not support it](https://learn.microsoft.com/aspnet/core/fundamentals/native-aot#aspnet-core-and-native-aot-compatibility).
- **Runtime**: the Web SDK defaults, spelled out in `Harbor.csproj`: Server GC (with DATAS, the .NET 9+ default), concurrent GC, tiered compilation with dynamic PGO. Startup observed: about 0.4 to 0.9 s to the first `/health` response on a busy machine, idle memory about 125 MiB.
- **Hosting**: Kestrel directly on `127.0.0.1:$PORT`, `ASPNETCORE_ENVIRONMENT=Production`, plain HTTP. `appsettings.json` sets the default log level to `Warning`, so nothing is logged per request. The only log line is the HTTPS redirection middleware's one-time warning that it found no HTTPS port.
- **Middleware**: the pipeline of the Razor Pages template (`dotnet new webapp`) is kept as is: `UseExceptionHandler` and `UseHsts` outside Development, `UseHttpsRedirection`, `UseRouting`, `UseAuthorization`, `MapStaticAssets`, `MapRazorPages().WithStaticAssets()`. Nothing is removed. HSTS only applies to HTTPS responses and the redirection does nothing without an HTTPS endpoint, so neither changes plain HTTP. Added: `UseStatusCodePagesWithReExecute` for the error pages, `UseResponseCompression` when enabled.
- **Build servers**: `--disable-build-servers` keeps the MSBuild and Roslyn servers from staying alive after the build and competing with the measurements.

## Threads and compression

- `BENCH_THREADS` unset or `0`: the .NET defaults for all cores.
- `BENCH_THREADS=N`: `run.sh` sets `DOTNET_PROCESSOR_COUNT=N` before the runtime starts. That is the runtime's override for `Environment.ProcessorCount` (the value it derives from a container's CPU limit), so .NET sizes itself for N CPUs: thread pool, GC (below two CPUs the runtime uses workstation GC) and Kestrel's I/O queues. Outside a container nothing enforces the limit, and the thread pool's hill climbing then adds threads on the idle cores (measured: about 3 cores busy with `DOTNET_PROCESSOR_COUNT=1` alone). `run.sh` therefore also fixes the pool at N worker threads with the [documented thread pool settings](https://learn.microsoft.com/dotnet/core/runtime-config/threading#maximum-threads) `DOTNET_ThreadPool_ForceMinWorkerThreads` / `DOTNET_ThreadPool_ForceMaxWorkerThreads` (.NET 10, hexadecimal values). Measured under load: 1.0 core of CPU time per second with `BENCH_THREADS=1`, 1.7 cores with `BENCH_THREADS=2`.
- `BENCH_COMPRESSION=on`: [`AddResponseCompression()` and `UseResponseCompression()`](https://learn.microsoft.com/aspnet/core/performance/response-compression) with the defaults (Brotli and Gzip at `CompressionLevel.Fastest`, default MIME types). Static assets are served precompressed by `MapStaticAssets` regardless of this setting, which is the framework's standard behavior.

## Layout

- `Program.cs`: services and the middleware pipeline.
- `Catalog/`: the dataset types, the `Store` singleton that loads the JSON files and builds every index at startup, the listing query (`Listing`), and the `System.Text.Json` source-generation context.
- `Pages/`: one Razor Page per HTML route (`@page "/products"` and so on), the error page, and `Shared/` with `_Layout` and the partials (`_Header`, `_Footer`, `_ProductCard`, `_ProductImage`, `_Breadcrumbs`, `_Pagination`).
- `TagHelpers/StarRowTagHelper.cs`: `<star-row product="..." />` and `<star-row review="..." />`.
- `Api/ApiEndpoints.cs`: the JSON API as a Minimal API route group.
- `Infrastructure/`: formatting, the cart cookie, URL builders, the color palette and the inline SVG icons.
- `Styles/app.css`: Tailwind input; the output goes to `wwwroot/css/app.css` (ignored by git).

## Judgment calls

- **Razor Pages over MVC.** Every HTML route is a page with a GET handler and no shared controller logic, which is what [Razor Pages is recommended for](https://learn.microsoft.com/aspnet/core/razor-pages/). Routes are given as absolute `@page` templates so the URLs match the spec exactly. The listing binds its parameters with `[FromQuery]` because `page` is a reserved route value in Razor Pages.
- **Components.** The product card, product image, breadcrumbs, pagination, header and footer are partials rendered with the `<partial>` tag helper. The star row is a tag helper with typed attributes (a `Product` or a `Review`), so Razor passes the object directly instead of rendering a string attribute into a buffer. The order status badge is a [Razor templated method](https://learn.microsoft.com/aspnet/core/mvc/views/razor#functions) in the orders page, its only user: it renders 500 times per request, and as a tag helper it cost about a quarter of that page's rendering time in an A/B measurement. Icons are static `HtmlString`s, like the reference's icon functions.
- **Error pages.** Pages return a bare `NotFound()` or `StatusCode(422)`, and [`UseStatusCodePagesWithReExecute`](https://learn.microsoft.com/aspnet/core/fundamentals/error-handling#usestatuscodepageswithreexecute) renders `Pages/Error.cshtml` in the shared layout, which also covers unknown paths. The API's JSON errors have bodies, so the middleware leaves them alone. The template's exception handler points at the same page (`/error/500`), with a generic message since the spec defines no 500 page.
- **Static assets.** [`MapStaticAssets`](https://learn.microsoft.com/aspnet/core/fundamentals/map-static-files) with `MapRazorPages().WithStaticAssets()`, as the .NET 9+ templates do. `asp-append-version` then emits build-time fingerprinted URLs (`/css/app.<hash>.css`) served with `Cache-Control: max-age=31536000, immutable`. The Tailwind target in `Harbor.csproj` runs before static web assets are collected, so the stylesheet is fingerprinted and precompressed like any other file in `wwwroot`. ASP.NET Core has no Tailwind integration of its own.
- **JSON.** Minimal API endpoints return `TypedResults`, with the source-generated `HarborJsonContext` first in the resolver chain of the HTTP JSON options and a snake_case naming policy. The framework's default relaxed escaping for HTTP responses applies. `NumberHandling` is set to `Strict` because the web defaults accept numbers in strings, which the cart API must reject. `POST /api/cart` reads the body with `ReadFromJsonAsync` instead of binding a parameter, so malformed bodies and wrong content types get the spec's 422 instead of the framework's 400/415. `required` members reject bodies without `product_id` or `quantity`. Minimal API JSON endpoints [do not run anti-forgery validation](https://learn.microsoft.com/aspnet/core/security/anti-request-forgery#anti-forgery-with-minimal-apis) (only form binding does), and Razor Pages only validates it for POST page handlers, of which there are none.
- **HTML encoding.** `WebEncoderOptions` allows all of Unicode, as the [encoding docs](https://learn.microsoft.com/aspnet/core/security/cross-site-scripting#customizing-the-encoders) suggest for non-English text, so `Seán` is written as is rather than as `Se&#xE1;n`. Markup characters are still escaped.
- **Formatting.** .NET's standard format strings with an explicit `en-US` culture, per request: `"C"` for money (`$1,234.50`), `"N0"` for counts, `"F1"` for ratings, `"MMM d, yyyy"` and `"MMM d, yyyy HH:mm"` for dates; the ISO `datetime` attributes use the invariant culture. The default thread culture is set to invariant as well, so nothing depends on the machine's locale.
- **Links** come from Razor Pages URL generation: `asp-page` / `asp-route-*` / `asp-fragment` anchor tag helpers for fixed and per-product links, `asp-route` with the API endpoint's name for the footer's API link, and `Url.Page` where the values are computed (listing, sort, sidebar and pagination links, breadcrumbs). Route values that are not part of a page's template become query parameters in the order given, so category comes before sort. The one exception is `page`: Razor Pages uses that route value for the page name, so `Url.Page` and `asp-route-page` cannot produce a `page` query parameter. `Urls.Listing` appends it with `QueryHelpers.AddQueryString`, which keeps it last as the spec requires.
- **Cost of the standard facilities.** An earlier version used interpolated strings with a month table for dates, hand-split cents for money and `QueryBuilder` strings for links. In an A/B run against it (two threads each, on a busy shared machine, so rough), the listing page used about 15 to 20% more CPU per request with framework URL generation, and the orders page's difference from the standard format strings stayed within the noise (0 to 15%).
- **Data.** `Store` is registered as a singleton and loaded before the host starts: products by id in an array, categories by slug in a `FrozenDictionary`, the four sort orders per category and overall, featured products, orders newest first and the order totals. Requests only index and slice.
- **Differences from the reference that the parity check does not cover.** Routing is case-insensitive and tolerates a trailing slash (`/About/`), which is ASP.NET Core's default. A duplicated query parameter (`?page=1&page=2`) uses the first value in the HTML listing and fails validation (`invalid page`) in the API, where the reference answers 422 `invalid query` in both. A JSON array body for `POST /api/cart` is rejected (the reference's serde accepts `[1, 2]` as a struct).
