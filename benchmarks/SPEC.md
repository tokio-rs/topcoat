# Benchmark specification

This is the contract every benchmark app and the runner implement. If an app and this file disagree, the app is wrong. Sections marked **(reference)** are filled in by whoever builds the reference app and are then binding for all other apps.

## Goals and ground rules

The benchmark compares web frameworks serving the same realistic storefront, "Harbor Market". It must be fair enough to survive hostile scrutiny.

1. **Idiomatic first.** Write each app the way that framework's own docs and experienced users would. Use the framework's router, templating, layouts, components, JSON support and config conventions. No hand-rolled string concatenation, no bypassing the framework to win.
2. **Use offered optimizations that keep code readable.** Production mode, precompiled templates, the framework's recommended production server and settings (for example Puma workers + YJIT for Rails, `output: "standalone"` for Next.js, Topcoat's `class!` macro). Do not use tricks a normal team would not ship.
3. **No response caching.** No full-page, fragment, or HTTP caching of the benchmarked routes, and no static prerendering of them. Every request renders. Every HTML page depends on the request's `cart_count` cookie, so this is the natural behavior anyway.
4. **The data layer is not under test.** No database, no ORM. Each app loads the shared JSON files into memory at startup and builds whatever indexes it likes at startup (think "the database already did its job"). Per request, the data layer only does O(1) lookups and slices. Everything else is per request: routing, parameter and cookie parsing, validation, formatting (money, dates, ratings), rendering, serialization.
5. **Same output.** Every app renders the same visible content and looks the same in a browser. `bench verify` enforces this against the reference app.
6. **No access logging.** Per-request logs are disabled (or log level set above info) in every app. Errors still log.
7. **Pinned versions.** Use the latest stable release of each framework and commit lockfiles.
8. Never weaken a competitor to make Topcoat look good, and never special-case Topcoat. When in doubt about what is idiomatic, follow the framework's official docs and note the decision in the app's `README.md`.
9. **Standard formatters.** Format dates, and numbers where the platform has one, with the ecosystem's standard formatter whenever it can express the required output, even if a hand-written version is faster. Dates: `time`'s `format_description!` in Rust, `time.Format` in Go, `ToString(format, CultureInfo)` in .NET, `Calendar.strftime` in Elixir, `strftime`/`to_fs` in Ruby, the `date` filter in Django, `Intl.DateTimeFormat` in JavaScript. Money and thousands separators: the standard helper where one exists (`number_to_currency`, .NET `"C"`/`"N0"` with `en-US`, `Intl.NumberFormat`); a small hand-written function where the standard library has none (Rust, Go, Elixir, Python's format spec).
10. **Framework URL generation.** Build internal links with the framework's own URL generation where it has one (route helpers, `{% url %}`, `~p`, `href!`, tag helpers, typed routes). Hand-built strings are fine only where the stack has no such facility (Axum, Go `net/http`).
11. **Loading versus formatting.** Parsing the dataset into typed values (dates, numbers, references between records) is part of loading and happens at startup. Formatting is always per request. The data must be fully loaded before `/health` first answers.
12. **Default middleware stays.** Keep the default middleware and response headers the framework's generator sets up. Remove only pieces that need a database or session store the app does not have, or that produce behavior the SPEC excludes, and list every removal in the app's `README.md`.
13. **`BENCH_THREADS=N` means at most N cores.** The runner enforces this with CPU affinity where the OS allows and flags any measurement where the process tree used materially more than N cores.

Framework-required deviations from the reference markup are allowed when unavoidable and documented in the app's `README.md` (for example the Dioxus mount element owning `id="main"`, the `leptos-island { display: contents }` rule, unhashed Leptos `/pkg` asset names).

## Layout of `benchmarks/`

```
SPEC.md            this file
README.md          how to run, methodology, caveats
bench.toml         default configuration and profiles
data/              generate script + checked-in JSON dataset
apps/<id>/         one app per framework, each with app.toml and README.md
runner/            Rust CLI (`cargo bench-web`), report template
results/           gitignored output, one directory per run
```

App ids: `topcoat`, `axum-maud` (reference), `leptos`, `leptos-islands`, `dioxus`, `nextjs`, `rails`, `aspnet`, `go-templ`, `django`, `phoenix`.

## Dataset

`data/generate.mjs` deterministically (seeded PRNG, no dependencies) writes:

- `products.json`: 240 products. Fields: `id` (1..240), `slug`, `name`, `category` (slug, one of 12), `price_cents`, `compare_at_cents` (nullable, for sale badge), `rating` (one decimal, 1.0..5.0), `review_count`, `stock` (0 = sold out), `color` (one of 8 swatch names), `tagline`, `description` (2-3 paragraphs), `specs` (6-8 `{label, value}`), `created_at` (ISO date), `featured` (bool, exactly 8 true), `related` (4 product ids), `reviews` (6-10 `{author, rating, title, body, date}`).
- `categories.json`: 12 `{slug, name, description}`.
- `orders.json`: 500 orders `{id, number, customer, email, placed_at (ISO datetime, UTC), status (pending|paid|shipped|delivered|refunded|cancelled), items (count), total_cents, country}`.

Some strings must contain characters that need HTML escaping (`&`, `<`, `>`, `"`, `'`) and non-ASCII text, so escaping is exercised and checked.

## Runtime contract (environment variables)

| Variable | Meaning |
|----------|---------|
| `PORT` | TCP port to listen on, bind `127.0.0.1`. |
| `BENCH_DATA_DIR` | Absolute path to `benchmarks/data`. |
| `BENCH_THREADS` | Unset or `0`: framework default (all cores). `N`: use N worker threads, or N worker processes for runtimes that scale by process (Node, Ruby, Python). |
| `BENCH_COMPRESSION` | `off` (default) or `on`. `on` enables the framework's standard response-compression middleware with its default settings. |

Rust apps additionally expose cargo features `jemalloc` and `mimalloc` (neither default) that set the global allocator, and build with the `benchmark` profile (`inherits = "release"`, `codegen-units = 1`, `lto = "fat"`).

## Routes

All HTML responses are `text/html; charset=utf-8`, JSON is `application/json`. Unknown product ids and invalid input return 404 / 422 as noted.

| Scenario id | Request | What it stresses |
|-------------|---------|------------------|
| `plaintext` | `GET /health` → `ok` (`text/plain`) | Framework floor. Also the readiness probe. |
| `about` | `GET /about` | Small, mostly static page in the shared layout. |
| `home` | `GET /` | Hero, 12 category links, the 8 featured product cards. |
| `listing` | `GET /products?category=<slug>&sort=<s>&page=<n>` | 24 product cards per page, category sidebar with counts, sort links, pagination. `category` optional. `sort` in `newest` (default), `price-asc`, `price-desc`, `rating`. Benchmarked with `category=outdoor&sort=price-asc&page=1` and without parameters `page=3`. |
| `detail` | `GET /products/{id}` | Product, spec table, all reviews, 4 related cards, quantity stepper + add-to-cart. 404 if unknown. Benchmarked over a rotating set of ids. |
| `orders` | `GET /admin/orders` | Large page: table of all 500 orders, 8 columns, status badges, formatted money and dates, summary totals. |
| `api-list` | `GET /api/products?category=&sort=&page=` | Same query as `listing`, JSON `{items, page, page_count, total}`; items omit `description`, `specs`, `reviews`, `related`. |
| `api-detail` | `GET /api/products/{id}` | Full product JSON. 404 if unknown. |
| `api-cart` | `POST /api/cart` with JSON `{"product_id": n, "quantity": n}` | Parse + validate (product exists, 1 ≤ quantity ≤ min(stock, 10), else 422 with `{"error": "..."}`), respond `{product_id, name, quantity, unit_price_cents, line_total_cents}`. Stateless. |

Every HTML page uses one shared layout: header with nav and a cart badge showing the integer from the `cart_count` cookie (default 0, ignore garbage), footer with link columns.

### Interactivity

The app has exactly two interactive pieces, both client-side only, implemented with the framework's own idiomatic client-side story (signals/hydration, islands, client components, Stimulus, Topcoat's runtime, plain JS where the framework has no opinion):

1. Detail page: quantity stepper (−/+, clamped to 1..min(stock, 10)) and an "Add to cart" button that adds the quantity to the header cart badge and writes the `cart_count` cookie.
2. Header: a mobile menu toggle button that shows/hides the nav on narrow screens.

Nothing else ships client-side behavior. This keeps the client payload comparison (JS / WASM / CSS bytes) meaningful.

### Formatting rules

- Money: `$1,234.50` (always two decimals, thousands separators).
- Dates: `Mar 4, 2026`. Order timestamps: `Mar 4, 2026 14:05` (UTC).
- Ratings: one decimal (`4.0`), plus a 5-star row.
- Counts: thousands separators (`1,204 reviews`).

### Markup and styling **(reference)**

Styling is Tailwind CSS v4 utility classes, integrated the way each framework normally does it. Class strings chosen by data (swatch color, order status, sale/sold-out badges) are selected from literal class strings in code so Tailwind can see them.

The reference app defines the exact DOM structure and class names in `apps/axum-maud` and documents the page anatomy below. Other apps port the markup one-to-one. Framework-inserted extras (hydration markers, script tags, comment nodes) are expected and fine.

The source of truth is the reference app: shared components live in `apps/axum-maud/src/views/` (one file each) and pages in `apps/axum-maud/src/pages/` (one file each). This section names every component, the data that feeds it, all data-dependent class strings, all static copy that the parity check compares, and every edge case. For anything not spelled out here, such as the exact class string of a wrapper `div`, copy it from the templates verbatim and check against the running reference app (`cargo run --release` in `apps/axum-maud`, then curl it).

#### Porting rules

- Keep element types and nesting. The parity check counts `a`, `article`, `tr`, `td`, `li`, `button`, `h1`, `h2` and `h3`, so never swap one of these for another element or add extra ones.
- Copy class strings verbatim and in the same order. When a class string is built from a fixed part and a data-dependent part, the fixed part comes first, then a space, then the table value, for example `"size-3 rounded-full ring-1 ring-black/10 " + accent`.
- Copy static text exactly as written below, including punctuation. All static copy is ASCII. Data strings contain non-ASCII and characters that need escaping; render them with the framework's normal escaping, never as raw HTML.
- Use the default Tailwind v4 theme with no customization. The reference stylesheet input is just `@import "tailwindcss"` plus the source path; no custom CSS exists.
- Serve CSS and JS under cacheable URLs the way the framework normally does (the reference uses content-hashed paths with `Cache-Control: public, max-age=31536000, immutable`). Add `<link rel="icon" href="data:,">` so browsers do not request `/favicon.ico`.
- Inline SVG icons are copied as-is; they are decorative (`aria-hidden="true"`) and not part of the text comparison, but keep them so page weight and rendering match.

#### Formatting details

- Money is integer cents formatted as `$` + dollars with `,` every three digits + `.` + two-digit cents: `5` is `$0.05`, `123450` is `$1,234.50`.
- Counts use `,` every three digits: `1,204`. Small integers such as page numbers, stock and item counts are written plainly.
- Ratings are written with one decimal: `4.0`, `4.3`. The number of filled stars is the rating rounded half up: with `t` the rating in tenths (`43` for 4.3), filled stars are `(t + 5) / 10` in integer division, so 4.5 fills 5 stars and 4.4 fills 4. Do not use banker's rounding (Python's `round`) or float rounding of the rating itself.
- Dates (`created_at`, review `date`) are `Mar 4, 2026`: three-letter English month, day without padding, year. The `datetime` attribute is the ISO date `2026-03-04`.
- Order timestamps are UTC, `Mar 4, 2026 14:05`: as above plus zero-padded 24-hour `HH:MM`, seconds dropped. The `datetime` attribute is `2026-03-04T14:05Z`.
- Description paragraphs are separated by a blank line (`\n\n`) in the data; each becomes one `p`.

#### Data order

- Featured products: the 8 products with `featured: true`, ascending id.
- Related products: the 4 ids in `related`, in data order.
- Reviews: data order (the data is already newest first).
- Orders: `placed_at` descending, ties by `id` descending.
- Categories everywhere: the order of `categories.json`.
- Listing sorts, all with ties broken by ascending `id`:
  - `newest`: `created_at` descending;
  - `price-asc`: `price_cents` ascending;
  - `price-desc`: `price_cents` descending;
  - `rating`: `rating` descending, then `review_count` descending.

Category product counts, sort orders and the order totals are startup indexes (ground rule 4). Formatting them is per request.

#### Layout (`views/layout.rs`)

Every HTML page, including error pages:

- `<!DOCTYPE html>`, `html lang="en"`, and in `head`: `meta charset="utf-8"`, `meta name="viewport" content="width=device-width, initial-scale=1"`, the `title`, the icon link, the stylesheet, and the script (deferred, or however the framework loads its client code).
- `title`: `<page title> | Harbor Market`, or just `Harbor Market` on the home page. Page titles are listed with each page.
- `body class="flex min-h-screen flex-col bg-stone-50 text-stone-900 antialiased"` containing, in order: a skip link `a href="#main"` with text `Skip to content` (visually hidden until focused), the header, `main id="main" class="flex-1"` with the page content, and the footer.

#### Header (`views/header.rs`)

Fed by the cart count only.

- Announcement bar `div`: `Free shipping on orders over $75 and free 30-day returns`.
- `header` (sticky), containing a row with:
  - Logo link to `/`: the logo SVG, then `Harbor Market`.
  - `nav id="site-nav" aria-label="Main"` with classes `order-last hidden w-full md:order-none md:block md:w-auto`, holding a `ul` of five `li > a`: `All products` (`/products`), `Outdoor` (`/products?category=outdoor`), `Kitchen` (`/products?category=kitchen`), `Electronics` (`/products?category=electronics`), `About` (`/about`).
  - Cart pill (a `div`, not a link): bag icon, `span class="hidden sm:inline"` with `Cart`, and the badge `span` (the reference marks it `data-cart-count`) with the cart count text.
  - Menu toggle `button type="button"` with `aria-controls="site-nav" aria-expanded="false"`, class `... md:hidden`, containing `span class="sr-only"` with `Toggle menu` and the menu icon.
- Cart count: read the `cart_count` cookie. It is valid only if it is 1 to 9 ASCII digits (leading zeros allowed, so `007` is 7); anything else, or no cookie, counts as 0. The badge shows the number, or `99+` when it is above 99.

Counts: 6 `a` (plus the skip link), 5 `li`, 1 `button`.

#### Footer (`views/footer.rs`)

Static.

- Brand column: logo link to `/` (logo SVG + `Harbor Market`) and `Well-made gear for every harbor day. Shipped from Portland, Oregon since 2016.`
- Three columns, each an `h2` and a `ul` of `li > a`:
  - `Shop`: `All products` (`/products`), `Outdoor & Camping` (`/products?category=outdoor`), `Kitchen & Dining` (`/products?category=kitchen`), `Audio` (`/products?category=audio`), `Toys & Games` (`/products?category=toys`).
  - `Help`: `Shipping & returns` (`/about#shipping`), `Warranty & repairs` (`/about#warranty`), `Contact us` (`/about#contact`).
  - `Company`: `About us` (`/about`), `Order admin` (`/admin/orders`), `Product API` (`/api/products`).
- Bottom row, two `p`: `Copyright 2026 Harbor Market. All rights reserved.` and `A demo storefront for comparing web frameworks.`

Every page therefore has at least 19 `a`, 16 `li`, 3 `h2` and 1 `button` from the layout.

#### Product image (`views/product_image.rs`)

The dataset has no photos. The tile is `div class="relative aspect-square overflow-hidden rounded-2xl " + tile`, containing one decorative shape `div` (`... rotate-6 rounded-[28%] shadow-xl transition group-hover:rotate-12 " + accent`) and, if any badge applies, a badge row `div class="absolute left-3 top-3 flex gap-1.5"` with, in this order:

- `Sold out` when `stock == 0`: `span class="rounded-full bg-stone-900 px-2.5 py-1 text-xs font-medium text-white"`.
- `Sale` when `compare_at_cents` is not null: `span class="rounded-full bg-rose-600 px-2.5 py-1 text-xs font-medium text-white"`.

Both badges can show at once. The badge row is omitted when neither applies.

Color palette (`views/palette.rs`), keyed by the product's `color`. The swatch dots use `accent` too.

| `color` | Label | `tile` | `accent` |
|---------|-------|--------|----------|
| `charcoal` | `Charcoal` | `bg-stone-200` | `bg-stone-700` |
| `sand` | `Sand` | `bg-amber-100` | `bg-amber-300` |
| `sage` | `Sage` | `bg-green-100` | `bg-green-700/70` |
| `navy` | `Navy` | `bg-blue-100` | `bg-blue-900` |
| `terracotta` | `Terracotta` | `bg-orange-100` | `bg-orange-700` |
| `ochre` | `Ochre` | `bg-yellow-100` | `bg-yellow-600` |
| `plum` | `Plum` | `bg-purple-100` | `bg-purple-800` |
| `sky` | `Sky` | `bg-sky-100` | `bg-sky-500` |

#### Star row (`views/stars.rs`)

`span class="flex items-center gap-0.5" role="img" aria-label="Rated <rating> out of 5"` containing five `svg viewBox="0 0 20 20"`, each with one `path` (the star). The first `filled` stars get the filled class, the rest the empty class:

| Size | Filled | Empty | Used by |
|------|--------|-------|---------|
| small | `size-4 fill-amber-400` | `size-4 fill-stone-300` | product cards, individual reviews |
| large | `size-5 fill-amber-400` | `size-5 fill-stone-300` | detail page header and review summary |

`<rating>` is the one-decimal product rating (`4.3`) or the integer review rating (`4`).

#### Product card (`views/product_card.rs`)

Fed by one product and its category. Used on the home page, the listing and the detail page.

```
article.group.relative.flex.flex-col
  product image
  div.mt-4.flex.flex-1.flex-col
    p      category name
    h3     a href="/products/<id>" > (span.absolute.inset-0, empty) + product name
    p      tagline (line-clamp-2)
    div    small star row + span "<rating> (<review_count>)"   e.g. "4.3 (1,204)"
    div    p > span price [+ s compare-at price]  |  span > (swatch span, empty) + color label
    p      "Only <stock> left"                                  only when 1 <= stock <= 9
```

The empty `span.absolute.inset-0` makes the whole card clickable. Counts per card: 1 `article`, 1 `h3`, 1 `a`.

#### Breadcrumbs (`views/breadcrumbs.rs`)

`nav aria-label="Breadcrumb"` > `ol` of `li`. Each link `li` holds an `a` and a chevron SVG; the last `li` holds `span aria-current="page"` with the current page's label.

#### Pagination (`views/pagination.rs`)

Rendered only when the listing has more than one page. `nav aria-label="Pagination"` containing, in order:

- `Previous`: an `a` to page - 1, or on the first page a `span aria-disabled="true"` with the disabled classes. Arrow SVG before the text.
- `p class="text-sm text-stone-600 sm:hidden"`: `Page <page> of <page_count>`.
- `ul class="hidden items-center gap-1 sm:flex"` with one `li > a` per page, every page from 1 to `page_count`. The current page's `a` has `aria-current="page"` and the current classes.
- `Next`: an `a` to page + 1, or on the last page a disabled `span`. Arrow SVG after the text.

Class strings (`STEP`, `STEP_DISABLED`, `PAGE`, `PAGE_CURRENT`) are constants at the top of the file.

#### Status badge (`views/status_badge.rs`)

`span class="inline-flex rounded-full px-2 py-0.5 text-xs font-medium ring-1 ring-inset " + colors`, text is the label.

| `status` | Label | `colors` |
|----------|-------|----------|
| `pending` | `Pending` | `bg-amber-50 text-amber-800 ring-amber-600/20` |
| `paid` | `Paid` | `bg-sky-50 text-sky-800 ring-sky-600/20` |
| `shipped` | `Shipped` | `bg-indigo-50 text-indigo-700 ring-indigo-600/20` |
| `delivered` | `Delivered` | `bg-emerald-50 text-emerald-800 ring-emerald-600/20` |
| `refunded` | `Refunded` | `bg-stone-100 text-stone-700 ring-stone-500/20` |
| `cancelled` | `Cancelled` | `bg-rose-50 text-rose-700 ring-rose-600/20` |

#### Home: `GET /` (`pages/home.rs`)

Title: none (`Harbor Market`). Fed by the categories with their product counts and the featured products.

1. Hero `section` (teal background): `p` `New season, same harbor`; `h1` `Well-made gear for every harbor day.`; `p` `Tents, knives, boots and blankets from small workshops we know by name. Built to last, easy to repair and shipped free over $75.`; links `Shop all products` (`/products`) and `Browse outdoor gear` (`/products?category=outdoor`); then a decorative `div aria-hidden="true"` with three colored shapes (desktop only).
2. Perks `section aria-label="Why Harbor Market"`: `ul` of three `li`, each an icon and two `p` (title, text):
   - `Free shipping over $75` / `Orders ship within one business day from Portland.` (truck icon)
   - `30-day returns` / `Changed your mind? Send it back, on us.` (refresh icon)
   - `Repair, don't replace` / `Spare parts and repairs for as long as we sell it.` (shield icon)
3. Categories `section`: `h2` `Shop by category`, then a `ul` with one `li > a` per category, linking to `/products?category=<slug>`. The `a` contains `h3` (name), `p` (description) and `p` `<count> products`.
4. Featured `section`: `h2` `Featured products`, link `View all` (`/products`, arrow icon after the text), then a grid `div` of the 8 featured product cards.

Counts: 42 `a`, 8 `article`, 31 `li`, 1 `button`, 1 `h1`, 5 `h2`, 20 `h3`.

#### About: `GET /about` (`pages/about.rs`)

Title: `About`. Static. Eyebrow `Our story`, `h1` `About Harbor Market`, a lead paragraph, a `dl` of four fact cards (`Founded` `2016`, `Workshops` `38`, `Repairs last year` `2,140`, `Orders shipped` `96,000+`), then three `section`s with ids `shipping` (`h2` `Shipping & returns`, two `p`), `warranty` (`h2` `Warranty & repairs`, a `p` and a three-item `ul`) and `contact` (`h2` `Contact us`, a `p` and a `dl` with `Email` / `help@harbor-market.example` and `Visit` / `1200 NW Front Ave, Portland, OR 97209`). Copy the paragraph text from the template.

#### Listing: `GET /products` (`pages/listing.rs`)

Query parameters `category`, `sort`, `page`; an empty value counts as absent. They are validated in this order, and the first failure decides the response:

| Condition | HTML | API (`/api/products`) |
|-----------|------|------------------------|
| `category` is not a known slug | 404 page | 404 `{"error": "category not found"}` |
| `sort` is not `newest`, `price-asc`, `price-desc` or `rating` | 422 page | 422 `{"error": "invalid sort"}` |
| `page` has a character other than `0`-`9`, or is `0` | 422 page | 422 `{"error": "invalid page"}` |
| `page` is above `page_count` (including numbers too large to parse) | 404 page | 404 `{"error": "page not found"}` |

Out-of-range pages are a 404, never clamped. `page_count` is `max(1, ceil(total / 24))`, so page 1 of an empty category is valid. The query parser's own failures (for example a duplicate parameter, if the framework rejects those) give 422 `{"error": "invalid query"}` in the API and the 422 page in HTML; this case is not verified.

Title and `h1`: the category name, or `All products` without a category. The description below the `h1` is the category description, or `Everything in the Harbor Market catalog, from tents to turntables.`

Links built by the listing omit parameters at their default (no category, `sort=newest`, `page=1`) and order them `category`, `sort`, `page`: `/products?category=outdoor&sort=price-asc&page=2`, `/products?page=3`. Category links keep the current sort and reset the page; sort links keep the current category and reset the page; pagination links keep both.

Content, in order:

1. Breadcrumbs: with a category, `Home` (`/`) > `Products` (`/products`) > current category name; without, `Home` (`/`) > current `Products`.
2. Heading row: `h1`, description `p`, and the result count `p`: `Showing <first> to <last> of <total> products` (for example `Showing 25 to 48 of 240 products`; `<total>` uses thousands separators), or `0 products` when the listing is empty.
3. Sidebar `aside`: `h2` `Categories`, then a `ul` of `li > a`: first `All products` with the total product count, then every category name with its count. Each count is a `span` after the name. The current entry (no category, or the selected one) has `aria-current="page"`, the `SIDEBAR_LINK_CURRENT` classes and a `text-teal-700` count; the others have `SIDEBAR_LINK` and a `text-stone-400` count.
4. Main column: `h2` `Sort by`, then a `ul` of four `li > a` labelled `Newest`, `Price: low to high`, `Price: high to low`, `Top rated`. The current one has `aria-current="true"` and `SORT_LINK_CURRENT`, the others `SORT_LINK`.
5. The grid `div` of up to 24 product cards, or, when the listing is empty, `p` `No products in this category yet.` instead.
6. Pagination, when there is more than one page.

Counts for `/products` (page 1 of 10): 72 `a`, 24 `article`, 45 `li`, 1 `h1`, 5 `h2`, 24 `h3`.

#### Detail: `GET /products/{id}` (`pages/detail.rs`)

Any path segment that is not the decimal id of an existing product returns the 404 page (`0`, `241`, `abc`). Title: the product name.

1. Breadcrumbs: `Home` > `Products` > category name (`/products?category=<slug>`) > current product name.
2. Two columns: the product image (sticky on desktop), and the info column:
   - Category link (`a`, category name, to the category listing).
   - `h1` product name.
   - Rating row: large star row, `span` with the rating, `a href="#reviews"` with `<review_count> reviews`.
   - Price row: `p` price; if on sale, `s` compare-at price and `span` `Save <compare_at - price> (<percent>%)` where `percent = (compare_at - price) * 100 / compare_at` in integer division (floor). Example: `Save $41.99 (26%)`.
   - `p` tagline.
   - `dl`: `dt` `Color` / `dd` with a swatch `span` and the color label; `dt` `Availability` / `dd` with one of `Sold out` (`font-medium text-stone-500`) when stock is 0, `Only <stock> left in stock` (`font-medium text-amber-700`) when stock is 1 to 9, or `In stock` (`font-medium text-emerald-700`) from 10 up.
   - Purchase box, see below.
   - `ul` of three `li`, each a check icon and text: `Free shipping on orders over $75`, `Free 30-day returns`, `Ships within one business day`.
3. Description `section`: `h2` `Description`, one `p` per paragraph. Specifications `section`: `h2` `Specifications`, a `table` whose `tbody` has one `tr` per spec with `th scope="row"` (label) and `td` (value).
4. Reviews `section id="reviews"`: summary column with `h2` `Customer reviews`, `p` rating, large star row, `p` `Based on <review_count> reviews`, `p` `Showing the <number of reviews> most recent.`; then a `ul` with one `li > article` per review: small star row and `time datetime="<iso date>"` with the date, `h3` title, `p` body, `p` author.
5. Related `section`: `h2` `You might also like` and a grid of the 4 related product cards.

Purchase box when `stock > 0`, with `max = min(stock, 10)`:

```
div data-product-form data-max="<max>"
  div
    div                                     stepper pill
      button type="button" aria-label="Decrease quantity" disabled     minus icon
      output aria-live="polite" aria-label="Quantity"  -> text "1"
      button type="button" aria-label="Increase quantity"              plus icon; disabled when max == 1
    button type="button"  -> "Add to cart"
  p role="status"                           empty
```

When sold out, the purchase box is only `div > button type="button" disabled` with text `Sold out` (1 button instead of 3). The `data-*` attributes are hooks for the reference's plain JS; frameworks with their own client story wire the same elements their way.

Counts for an in-stock product with 10 reviews and 7 specs: 28 `a`, 14 `article`, 7 `tr`, 7 `td`, 4 `button`, 1 `h1`, 7 `h2`, 14 `h3`.

#### Orders: `GET /admin/orders` (`pages/orders.rs`)

Title: `Orders`.

1. `p` `Admin`, `h1` `Orders`, `p` `All 500 orders, newest first.` (the count with thousands separators).
2. `dl` of four cards, each a `div` with `dt` label, `dd` value and `dd` note:

   | Label | Value | Note |
   |-------|-------|------|
   | `Orders` | number of orders (count) | `All time` |
   | `Revenue` | sum of `total_cents` over `paid`, `shipped` and `delivered` orders (money) | `Paid, shipped and delivered` |
   | `Average order` | revenue divided by the number of those orders, rounded half up to a cent (money) | `Across revenue orders` |
   | `Refunded` | sum of `total_cents` over `refunded` orders (money) | `Returned to customers` |

   With the checked-in data: `500`, `$236,143.62`, `$540.37`, `$13,417.85`.
3. A `table` with `thead > tr > th scope="col"` for `Order`, `Placed`, `Customer`, `Email`, `Country`, `Items`, `Status`, `Total` (`Items` and `Total` right-aligned), and a `tbody` with one `tr` per order and eight `td`: `number`, `time datetime="<iso datetime>"` with the formatted timestamp, `customer`, `email`, `country`, `items`, the status badge, and the money total. Shared classes sit on the `table` (`whitespace-nowrap`) and `tbody` (`text-stone-600`); cells only add padding and their differences.

Counts: 501 `tr`, 4000 `td`.

#### Error pages (`pages/error.rs`)

Rendered in the full layout, so the cart badge works. Both have a `p` with the status code, an `h1`, a `p` message, and two links: `Back to home` (`/`) and `Browse products` (`/products`).

| Status | Title and `h1` | Message |
|--------|----------------|---------|
| 404 | `Page not found` | `Sorry, we couldn't find the page you're looking for.` |
| 422 | `Invalid request` | `Some of the filters in this link aren't valid. Try browsing from the catalog instead.` |

Any unknown path, including under `/api`, gets the HTML 404 page.

#### JSON API (`api/`)

All responses are `application/json`. Error bodies are `{"error": "<message>"}` with the messages below; use them verbatim even though the parity check only requires a string `error`.

- `GET /api/products`: `{"items": [...], "page": 1, "page_count": 10, "total": 240}`. Each item has `id`, `slug`, `name`, `category`, `price_cents`, `compare_at_cents`, `rating`, `review_count`, `stock`, `color`, `tagline`, `created_at`, `featured`, with values exactly as in `products.json`. Validation and errors as in the listing table above.
- `GET /api/products/{id}`: the product object exactly as in `products.json`, all fields. Unknown or non-numeric id: 404 `{"error": "product not found"}`.
- `POST /api/cart` with a JSON body. Checks, in order; every failure is 422:
  1. The body must be a JSON object with integer `product_id` and `quantity` (extra fields are ignored). Otherwise `invalid request body`. This also covers malformed JSON, a missing or wrong `Content-Type`, and strings or nulls in either field. Whether `2.0` counts as an integer is not verified.
  2. `product_id` must be an existing product: `product not found`.
  3. The product must have `stock > 0`: `product is sold out`.
  4. `1 <= quantity <= min(stock, 10)`: `quantity must be between 1 and <min(stock, 10)>`, for example `quantity must be between 1 and 10`.

  Success is 200 `{"product_id": 1, "name": "...", "quantity": 2, "unit_price_cents": 36895, "line_total_cents": 73790}`, where `line_total_cents = price_cents * quantity`. Nothing is stored.

#### Client-side behavior

The reference ships `assets/app.js` (about 2 KB, plain JS, loaded with `defer` on every page). Ports implement the same behavior with their own client-side story.

Mobile menu: clicking the toggle flips its `aria-expanded` between `"false"` and `"true"` and removes or adds the `hidden` class on `#site-nav`. On `md` screens and up the nav is always visible through `md:block`, and the toggle is hidden.

Quantity stepper and add to cart, on the detail page of an in-stock product:

- The quantity starts at 1. Minus and plus change it by one, clamped to `1..max` with `max = min(stock, 10)`. After every change the `output` shows the quantity, the minus button is disabled at 1 and the plus button is disabled at `max`.
- Add to cart reads the current count from the `cart_count` cookie with the same rule as the server (1 to 9 digits, else 0), adds the quantity, caps the sum at `999999999`, and writes `cart_count=<sum>; Path=/; Max-Age=2592000; SameSite=Lax`. It then sets the badge text to the sum (or `99+` above 99) and the `role="status"` paragraph to `Added <quantity> to your cart.`. The quantity is not reset.

Nothing else runs on the client.

#### Escaping probes in the data

Products 1 and 2 carry fixed reviews with `<script>alert('hi')</script>` as an author, `<script>` in a body, and `&`, `<`, `>`, `"` and `'` in titles and bodies. Orders 42 and 288 have the customers `Brown & Daughters <Wholesale>` and `Cafe "Le Phare"` (with an accented e), category names contain `&`, many taglines contain `&`, `<`, `>` or `"`, and names, countries and authors use non-ASCII letters. They must all render as text.

## App manifest: `apps/<id>/app.toml`

Every app has an `app.toml`. The runner reads nothing else from the app, so everything it needs to build, start and describe the app goes here. Unknown keys are an error, which catches typos. A manifest that fails to parse or validate makes the runner warn and skip that app; it is an error only when the app was requested explicitly with `--only`.

```toml
id = "axum-maud"                     # must equal the directory name
name = "Axum + Maud"                 # display name
language = "Rust"
kind = "rust"                        # "rust" enables allocator features and the benchmark profile
rendering = "ssr"                    # ssr | ssr+hydration | islands | rsc+hydration
description = "One line on the stack: server, templating, client-side approach."
requires = ["cargo"]                 # commands that must be on PATH, else the app is skipped with a warning
version_command = "..."              # optional: prints the framework version for the report
artifacts = ["target/{profile}/harbor-axum-maud"]  # optional: files or directories whose size is reported

[build]
# Run in order from the app directory with `sh -c`. Placeholders are substituted first.
commands = ["cargo build --profile {profile} {features}"]
env = { }                            # optional: extra environment for the build commands

[run]
command = "target/{profile}/harbor-axum-maud"
env = { }                            # extra env; PORT/BENCH_* are always set by the runner
ready_timeout_secs = 60
stop_timeout_secs = 10

[client]
# Extra URLs the browser loads that are not discoverable from the HTML (for example a WASM file fetched by JS).
extra_assets = []
```

| Field | Required | Meaning |
|-------|----------|---------|
| `id` | yes | App id, equal to the directory name under `apps/`. Used by `--only` and `--skip`. |
| `name` | yes | Display name in the terminal and the report. |
| `language` | yes | Implementation language, shown in the report. |
| `kind` | yes | `rust` for Rust apps; any other value (`node`, `ruby`, `dotnet`, `go`, `python`, `elixir`) is informational. For `rust`, `{profile}` is `benchmark` and `{features}` selects the allocator. |
| `rendering` | yes | How the app renders and what it ships to the browser: `ssr` (server-rendered, small or no client runtime), `ssr+hydration` (a client runtime hydrates the whole page), `islands` (only interactive islands hydrate), `rsc+hydration` (React Server Components plus hydrated client components). Shown next to the app's name everywhere in the report. |
| `description` | yes | One line on the stack. |
| `requires` | no | Commands that must be on `PATH`. If one is missing, `run` and `verify` skip the app with a warning and `doctor` lists it. Their `--version` output is recorded in the results. |
| `version_command` | no | Shell command run in the app directory after the build; its output (whitespace collapsed, at most 200 characters) is reported as the framework version. Read it from the lockfile where possible so it shows the exact version. |
| `artifacts` | no | Paths relative to the app directory, with placeholders, whose total size is reported after the build. Directories count recursively. List the deployable build output (a binary, a publish directory), not dependencies installed elsewhere. |
| `build.commands` | no | Build commands, run in order from the app directory with `sh -c`. Output goes to `results/<run>/logs/<id>.build.log`. A failing command marks the app `build_failed` and the run continues. Skipped with `--skip-build`. |
| `build.env` | no | Extra environment for the build commands and `version_command`. Values may use placeholders. The `BENCH_*` variables (without `PORT`) are set too. |
| `run.command` | yes | Starts the server. It runs from the app directory as `sh -c "exec <command>"`, so `$PORT` and other variables expand, and the server replaces the shell. It must be a single command; use a script for anything more. The server must run in the foreground and serve on `127.0.0.1:$PORT`. Forked workers are fine: the runner measures and stops the whole process group. |
| `run.env` | no | Extra environment for the server. Values may use placeholders. `PORT` and `BENCH_*` from the runner take precedence. |
| `run.ready_timeout_secs` | no | How long `GET /health` may take to answer `200` after the process starts. Default 60. |
| `run.stop_timeout_secs` | no | How long the server gets to exit after `SIGTERM` before the group is killed. Default 10. |
| `client.extra_assets` | no | URL paths the browser downloads that neither the HTML nor a script references as a string literal, for example a chunk loaded by computed name. They count toward the client payload of every measured page. |

Placeholders, substituted in `build.commands`, `build.env` values, `run.command`, `run.env` values, `version_command` and `artifacts`:

| Placeholder | Rust apps | Other apps |
|-------------|-----------|------------|
| `{app_dir}` | absolute path of the app directory | the same |
| `{profile}` | `benchmark` | `release` |
| `{features}` | `--features jemalloc`, `--features mimalloc` or empty for `system`, from the `allocator` setting | empty |
| `{allocator}` | `jemalloc`, `mimalloc` or `system` | `default` |

Commands run with the runner's environment minus the variables cargo injects into the processes it runs (`CARGO`, `CARGO_MANIFEST_DIR`, `CARGO_PKG_*`, `CARGO_BIN_*`, `CARGO_CRATE_NAME`, `CARGO_PRIMARY_PACKAGE`, `OUT_DIR` and similar), because the runner itself is started through `cargo run`. User configuration such as `CARGO_HOME`, `CARGO_TARGET_DIR`, `CARGO_BUILD_*`, `CARGO_NET_*` and `RUSTUP_*` is kept.

With `BENCH_THREADS=N` (N > 0) on Linux, the server is started under `taskset -c 0-(N-1)` when `taskset` is available, so the whole process group is pinned to N CPUs. On every platform, a measurement whose process tree kept materially more than N cores busy is flagged as exceeding the thread limit and is not compared (see `thread_limit_margin` in `bench.toml`).

Apps should keep their dependencies inside the app directory (`node_modules`, `vendor/bundle` via `BUNDLE_PATH` or `bundle config set path`, a uv-managed `.venv`, `deps` and `_build`), so that runs do not depend on anything installed in the user's home directory.

## Parity check

`cargo bench-web verify` builds and starts the reference app (`axum-maud`) and every selected app, one at a time, sends the same requests to each, and compares every response with the reference. It exits non-zero if any app fails. Use `--only` and `--skip` to select apps and `--skip-build` to reuse existing builds.

Requests, all with the cookie `cart_count=3` unless noted:

- every scenario URL: `/health`, `/about`, `/`, both listing queries, `/admin/orders`, both API listing queries, and a spread of product ids for the detail page and API (first, last, quartiles, an in-stock, a sold out and a discounted product),
- extra listing and API listing variants: no parameters, `sort=price-desc`, `sort=rating&page=2`, `sort=newest&page=10`, and a category with `sort=rating`,
- `POST /api/cart` with four valid bodies,
- `GET /` with `cart_count=7`, without a cookie, and with `cart_count=lots`,
- `GET /products/999999` and `GET /api/products/999999`, which must return `404`,
- `POST /api/cart` with quantity 0, quantity 11, an unknown product, a sold out product, a JSON array body and malformed JSON, which must return `422`.

Each response must have the expected status code, and so must the reference's. Then:

- **Content type.** HTML must be `text/html; charset=utf-8`, JSON `application/json`, plaintext `text/plain`. Parameters are compared case-insensitively; JSON and plaintext may add `charset=utf-8` and nothing else. `404` responses are only checked for the status.
- **HTML.** The visible text must be equal. Text is extracted in document order, ignoring comments and the content of `script`, `style`, `template`, `noscript` and inline `svg` elements, and split into lines at block-level elements. The comparison ignores whitespace entirely, because templating engines differ in the whitespace between tags; a failure prints a line diff against the reference. The counts of `a`, `article`, `tr`, `td`, `li`, `button`, `h1`, `h2` and `h3` elements must also be equal.
- **JSON.** The parsed values must be deeply equal, ignoring key order. Numbers compare by value, so `4` equals `4.0`. A failure lists the JSON paths that differ.
- **422 bodies.** Must be a JSON object with a string `error`. The message itself may differ between apps.
- **Plaintext.** The trimmed body must be equal.
- **Cookies set by pages.** A `GET` of an HTML page that answers with `Set-Cookie` produces a warning, not a failure: the pages are stateless, so such a cookie usually comes from a default middleware worth knowing about.
- **Cart cookie.** The visible text of `/` with `cart_count=7` must differ from the text without a cookie, so the badge actually reads the cookie. Comparing both with the reference checks the rendered value, and `cart_count=lots` checks that garbage falls back to 0 like the reference.
