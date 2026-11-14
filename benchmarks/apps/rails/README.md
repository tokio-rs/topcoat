# Harbor Market: Rails

The Harbor Market storefront from [`SPEC.md`](../../SPEC.md) as a standard Rails 8 app: generated with `rails new`, server-rendered ERB, and the default Hotwire front end.

## Stack

| Piece | Version |
|-------|---------|
| Ruby with YJIT | 3.4.9 (from the Nix shell) |
| Rails (Action Pack, Action View, Active Model; no Active Record) | 8.1.4.1 |
| Puma | 8.0.2 |
| Propshaft (asset pipeline) | 1.3.2 |
| importmap-rails | 2.2.3 |
| turbo-rails (Turbo 8) | 2.0.23 |
| stimulus-rails (Stimulus 3) | 1.3.4 |
| tailwindcss-rails, with tailwindcss-ruby pinned to Tailwind CSS 4.3.3 | 4.7.0 |
| bootsnap | 1.26.1 |

Gems are installed into `vendor/bundle` (`BUNDLE_DEPLOYMENT=1`) and `Gemfile.lock` is committed. The lockfile lists the Linux platforms too, so the same lock installs on a Linux benchmark machine.

The app was generated with:

```sh
rails new harbor --css tailwind --skip-active-record --skip-action-mailer --skip-action-mailbox \
  --skip-action-text --skip-active-job --skip-active-storage --skip-action-cable --skip-jbuilder \
  --skip-test --skip-system-test --skip-kamal --skip-thruster --skip-solid --skip-docker --skip-ci \
  --skip-rubocop --skip-brakeman --skip-bundler-audit --skip-devcontainer
```

followed by the importmap, Turbo, Stimulus and Tailwind installers that `rails new` runs after `bundle install`. The module is named `Harbor` because an app module named `Rails` would clash with the framework.

## Running

```sh
# From the repository root, inside `nix develop .#benchmarks`.
cd benchmarks/apps/rails
export BUNDLE_DEPLOYMENT=1 BUNDLE_WITHOUT=development:test RAILS_ENV=production
bundle install
SECRET_KEY_BASE_DUMMY=1 bin/rails assets:precompile
RAILS_LOG_LEVEL=warn SECRET_KEY_BASE=dummy PORT=3000 bundle exec puma -C config/puma.rb
```

For development, `bundle install` without the variables above and `bin/dev` (Puma plus `tailwindcss:watch`).

## Settings

| Benchmark setting | Rails setting |
|-------------------|---------------|
| `PORT` | `port ENV["PORT"], "127.0.0.1"` in `config/puma.rb`. |
| `BENCH_THREADS` unset or `0` | `workers :auto`: one Puma worker process per core, what `WEB_CONCURRENCY=auto` does. |
| `BENCH_THREADS=1` | Puma single mode: one process, no cluster. |
| `BENCH_THREADS=N` | `workers N`. |
| Threads per process | The template's default of 3 (`RAILS_MAX_THREADS`). |
| `BENCH_COMPRESSION=on` | `Rack::Deflater` with default settings, inserted before `ActionDispatch::Static` so assets are compressed too. Off otherwise. |
| `BENCH_DATA_DIR` | `config.x.data_dir`, read by `Catalog`. |

In cluster mode Puma uses `preload_app!`, as the [Puma docs](https://github.com/puma/puma/blob/master/docs/deployment.md) and the comments in Rails' `config/puma.rb` recommend: the master boots the app and loads the catalog, then forks, so workers share that memory copy-on-write.

Production settings, from `app.toml` and `config/environments/production.rb`:

- `RAILS_ENV=production`: eager loading, no code reloading, template caching.
- YJIT: Rails 8 turns it on in production (`config.yjit`, the default since `load_defaults 7.2`). Checked in the running server: `RubyVM::YJIT.enabled?` is `true` in single mode and in every cluster worker.
- `RAILS_LOG_LEVEL=warn`: no per-request log lines; errors are still logged.
- `SECRET_KEY_BASE` is a fixed dummy value in the manifest. The app signs and encrypts nothing, but Rails refuses to boot in production without one. The build uses `SECRET_KEY_BASE_DUMMY=1`, like the generated Dockerfile.
- `config.force_ssl` and `config.assume_ssl` are off: the template turns them on for a TLS-terminating proxy, and the benchmark talks plain HTTP to Puma on loopback. `config.hosts` stays empty (the template default), so host authorization lets `127.0.0.1` through.
- Static files: `bin/rails assets:precompile` writes digested files to `public/assets`, and `ActionDispatch::Static` (on by default since Rails 8) serves them with the template's `cache-control: public, max-age=31536000`. There is no reverse proxy and no Thruster, as instructed.
- The build follows the generated Dockerfile: `bundle install`, `bootsnap precompile`, `assets:precompile`.

## Layout

- `app/models/`: `Catalog` loads the three JSON files once and builds every index (categories by slug, per-category and global sort orders, featured products, orders newest first, the order summary). `Product`, `Category` and `Order` are frozen `Data` classes with dates parsed into `Date` and `Time`, like typed columns. `Listing` validates the listing parameters and slices a page; both the HTML page and the API use it.
- `config/initializers/catalog.rb`: loads the catalog in `after_initialize`, so it happens at boot (before Puma forks), not on the first request.
- `app/controllers/`: one controller per page, `Admin::OrdersController`, and `Api::` controllers on `ActionController::API`.
- `app/views/`: ERB with the application layout, header and footer partials, and partials for the product card, product image, star row, breadcrumbs, pagination, purchase box, review and order row.
- `app/javascript/controllers/`: the three Stimulus controllers.

## Judgment calls

- **Rendering.** Lists render with collection partials (`render partial: "products/card", collection:`), which look up the template once. Partials use strict locals (`<%# locals: (...) -%>`). Pages set the title with `content_for :title`. Links use `link_to` and route helpers.
- **No caching.** There is no fragment or Russian-doll caching on the benchmarked routes, per the ground rules. A real Rails app would very likely wrap the product cards and order rows in `cache` blocks and skip most of this rendering; the rule applies to every framework equally, so it is left out here.
- **Formatting.** Money uses `number_to_currency` and counts use `number_with_delimiter`, the standard Action View helpers. They are not cheap: they look up their options through I18n and round through `BigDecimal` on every call. On the development machine `number_to_currency` took about 22 microseconds per call, and swapping it for a few lines of plain Ruby cut the CPU time of the orders page (504 calls) from about 29 ms to 8 ms and of a listing page by about a third. The helper stays because it is what a Rails team would write. Dates use `to_fs` with formats registered in `config/initializers/time_formats.rb`. Ratings use `format("%.1f")`, since `number_with_precision` adds nothing for a one-decimal float.
- **Icons.** SVG icons are written inline in the templates, the way Tailwind markup is usually pasted into ERB. The logo is a partial because the header and footer share it. Building static icons with the `tag` helper cost about 10 microseconds each, for markup that never changes.
- **JSON.** `render json:` with explicit hashes: `Product#as_json` returns every field (so `render json: product` works) and `Product#summary_json` the list fields. No serializer gem; Jbuilder is skipped because it is much slower and the payloads are flat.
- **API controllers** inherit from `ActionController::API`, which has no CSRF protection, cookies or views, the documented way to build a JSON API in Rails. `POST /api/cart` reads the JSON body through `params` and rescues `ActionDispatch::Http::Parameters::ParseError` to answer 422 instead of the default 400.
- **Removed template defaults.** The template's layout calls `csrf_meta_tags`, which creates a session and sends an encrypted session cookie on every response. The store has no forms, no session and no non-GET browser requests, so the layout leaves it out. `allow_browser versions: :modern` is removed from `ApplicationController`: it parses the `User-Agent` of every request and answers old browsers with a 406, which a public storefront would not do. The PWA meta tags and icons go too; the SPEC asks for `<link rel="icon" href="data:,">`.
- **Kept defaults.** The default middleware stays, including `Rack::ETag`, which hashes every HTML body, plus the security headers and request IDs. `stale_when_importmap_changes` stays as generated.
- **Errors.** Unknown paths hit a catch-all route that renders the 404 page in the layout, so the cart badge still works. Listing validation raises `Listing::Error`, which carries its status and API message; HTML controllers `rescue_from` it into the 404 or 422 page and API controllers into JSON. Routes use `format: false` so `/products/1.json` is a 404 instead of a format switch.
- **Query order.** Listing links are built by `listing_path` in `ProductsHelper` instead of `products_path(...)`, because Rails sorts query parameters alphabetically and the SPEC orders them `category`, `sort`, `page`.
- **Tailwind.** `app/assets/tailwind/application.css` sets the sources explicitly (`app/views`, `app/helpers`) instead of relying on automatic detection, so the output matches the reference stylesheet's set of classes. tailwindcss-ruby is pinned to 4.3.3, the shell's Tailwind version.

## Client side

The Rails default: Turbo and Stimulus through importmap, no bundler. `javascript_importmap_tags` writes the import map and a `modulepreload` link for every pinned module.

- `menu_controller.js` toggles `aria-expanded` and the nav's `hidden` class.
- `product_form_controller.js` keeps the quantity in a Stimulus value, enables and disables the stepper buttons, writes the `cart_count` cookie and dispatches `product-form:added`.
- `cart_badge_controller.js` on the header badge listens for that event (`product-form:added@window`) and updates the count.

Turbo Drive is on, as in every new Rails app, so links navigate with `fetch` and swap the body. It is the bulk of the JavaScript a browser downloads. Uncompressed, the JS is about 157 KB: `turbo.min.js` 106 KB, `stimulus.min.js` 46 KB, `stimulus-loading.js` 3 KB, and a few hundred bytes for `application.js` and each controller. The stylesheet is about 33 KB. A team that did not want Turbo Drive could unpin it, but that is not the default stack.
