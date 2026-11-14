# Harbor Market: Django

The Harbor Market storefront from [`SPEC.md`](../../SPEC.md), written as a standard Django project and served by Gunicorn.

## Stack

| Piece | Version |
|-------|---------|
| Python (from the Nix shell) | 3.13.15 |
| [Django](https://www.djangoproject.com/), Django template language | 6.1.3 |
| [Gunicorn](https://gunicorn.org/), gthread workers | 27.0.0 |
| [WhiteNoise](https://whitenoise.readthedocs.io/) for static files | 6.12.0 |
| Tailwind CSS standalone CLI, run as a build step | the shell's `tailwindcss` (4.3.3) |
| Client-side behavior | `store/static/store/app.js`, plain JS, about 1.9 KB (the reference's script) |

Dependencies are managed with [uv](https://docs.astral.sh/uv/): `pyproject.toml` plus the committed `uv.lock`, installed into `.venv/` inside this directory.

## Running

```sh
uv sync --locked --no-dev
tailwindcss --input assets/app.css --output store/static/store/app.css --minify
.venv/bin/python manage.py collectstatic --noinput --clear
PORT=8000 BENCH_THREADS=4 BENCH_COMPRESSION=on .venv/bin/gunicorn --config gunicorn.conf.py harbor.wsgi
```

`BENCH_DATA_DIR` defaults to `../../data`.

## Layout

- `harbor/`: the project package (`settings.py`, `urls.py`, `wsgi.py`).
- `store/`: the one app.
  - `catalog.py`: loads the JSON files into dataclasses and builds every index once, at import time (category lookup, per-category and global sort orders, featured list, related products, orders newest first, order totals). `StoreConfig.ready()` imports it, so it is loaded at startup.
  - `listing.py`: query validation and listing links, shared by the listing page and the API.
  - `views.py` (HTML pages, function-based views with `render`), `api.py` (`JsonResponse`).
  - `context_processors.py`: the cart badge from the `cart_count` cookie, available to every template, including the 404 page.
  - `templatetags/storefront.py`: the `money` and `count` filters and the inclusion tags for the product card, product image, star row, breadcrumbs, pagination and status badge.
  - `templates/`: `store/base.html` with template inheritance, the pages, `store/includes/` (header, footer and the inclusion tag templates), `store/icons/` (inline SVGs pulled in with `{% include %}`), and the `404.html` and `422.html` error pages.
- `gunicorn.conf.py`: server settings from the benchmark environment.

## Server, threads and compression

- **Gunicorn with gthread workers, one process per core.** `BENCH_THREADS` unset or `0` starts `os.process_cpu_count()` worker processes; `N` starts `N`. Each worker runs one request thread (`threads = 1`), so `BENCH_THREADS=1` is a single process doing Python work on one core, and the GIL never makes threads within a worker compete for CPU.
- **Why gthread rather than the default sync worker.** Gunicorn's sync worker [does not support keep-alive](https://gunicorn.org/reference/settings/#keepalive) and closes the connection after every response; it is designed to sit behind a buffering proxy such as nginx, which Gunicorn's [deployment guide](https://gunicorn.org/deploy/) strongly recommends. This benchmark has no proxy: the load generator talks to Gunicorn directly over keep-alive connections, like it does with every other app. With sync workers every request opens a new TCP connection, the server-closed sockets pile up in `TIME_WAIT`, and on macOS the load generator runs out of ephemeral ports within a few scenarios (`Can't assign requested address (os error 49)`), which invalidates measurements. The gthread worker is Gunicorn's built-in worker for exactly this case: it keeps idle keep-alive connections in an event loop and hands readable ones to its thread pool. To run with sync workers anyway: `GUNICORN_CMD_ARGS="--worker-class sync"`.
- **One worker per core, not `2 * cores + 1`.** Gunicorn's FAQ suggests `(2 x $num_cores) + 1` workers so that some can wait on I/O while others compute. This app does no I/O per request (no database), so extra processes would only time-slice the same cores.
- **`preload_app = True`.** Django, the templates' Python code and the catalog are loaded once in the master and shared copy-on-write by the forked workers. Templates are compiled lazily in each worker on first use and then cached.
- **Compression.** `BENCH_COMPRESSION=on` adds Django's [`GZipMiddleware`](https://docs.djangoproject.com/en/6.1/ref/middleware/#module-django.middleware.gzip) with its defaults, placed after WhiteNoise (which serves its own precompressed static files) and before every middleware that touches the body, as the [ordering notes](https://docs.djangoproject.com/en/6.1/ref/middleware/#middleware-ordering) say.
- **No access log.** Gunicorn's `accesslog` is off by default and stays off. `LOGGING` sends only `ERROR` and above from the `django` logger to stderr, so 404s and 422s are not logged but server errors are. With `DEBUG = False` Django's default configuration is already silent; the setting makes that explicit.

## Settings and middleware

Production settings: `DEBUG = False`, `ALLOWED_HOSTS = ["127.0.0.1", "localhost"]`, `DATABASES = {}`.

Middleware, in order:

1. `django.middleware.security.SecurityMiddleware`
2. `whitenoise.middleware.WhiteNoiseMiddleware` (directly after `SecurityMiddleware`, as [WhiteNoise's docs](https://whitenoise.readthedocs.io/en/stable/django.html) require)
3. `django.middleware.gzip.GZipMiddleware`, only with `BENCH_COMPRESSION=on`
4. `django.middleware.common.CommonMiddleware`
5. `django.middleware.csrf.CsrfViewMiddleware`
6. `django.middleware.clickjacking.XFrameOptionsMiddleware`

Removed from the `startproject` defaults, because there is no database and nothing uses them:

| Removed | Why |
|---------|-----|
| `django.contrib.admin` | Needs `auth`, `contenttypes` and a database. The "Order admin" page is an ordinary view. |
| `django.contrib.auth` and `AuthenticationMiddleware` | No users or logins. The auth models need a database, and the middleware would attach a lazy `request.user` to every request for nothing. |
| `django.contrib.contenttypes` | Only needed by `auth` and `admin`; its models need a database. |
| `django.contrib.sessions` and `SessionMiddleware` | No sessions; the cart lives in a plain client-side cookie, per the SPEC. The default session backend is the database. |
| `django.contrib.messages` and `MessageMiddleware` | No flash messages, and the default storage falls back to sessions. |
| The `auth` and `messages` context processors | Their apps are gone. The `debug` context processor is dropped too, since it does nothing with `DEBUG = False`. |

Kept: the security, common, CSRF and clickjacking middleware a production site runs. `CsrfViewMiddleware` does no work on the `GET` pages because no template uses `{% csrf_token %}`; `POST /api/cart` is a JSON endpoint called without a CSRF token, so it is marked [`@csrf_exempt`](https://docs.djangoproject.com/en/6.1/howto/csrf/#disabling-csrf-protection-for-just-a-few-views).

`django.contrib.staticfiles` stays for `collectstatic` and the `{% static %}` tag.

## Judgment calls

- **Templates.** Django template language with inheritance (`base.html`, and `store/error.html` for the two error pages), `{% include %}` for the header, footer and icons, and inclusion tags for the components that take arguments. Class strings chosen by data are literal: in templates for branches, and in `palette.py` and `templatetags/storefront.py` for the color palette, star and status badge tables. Tailwind's `@source` scans `store/templates` and every Python file in `store/`.
- **Cached template loader.** `TEMPLATES` sets no `loaders`, and in that case Django [wraps the filesystem and app directories loaders in the cached loader](https://docs.djangoproject.com/en/6.1/ref/templates/api/#django.template.loaders.cached.Loader), so each template is parsed once per process.
- **URLs.** Templates link to routes with `{% url %}` and listing links are built with `reverse()` plus `urlencode`, rather than hard-coded paths, as the Django tutorial teaches. This costs a little per link.
- **Formatting.** Dates use the built-in [`date`](https://docs.djangoproject.com/en/6.1/ref/templates/builtins/#date) filter (`"M j, Y"`, `"M j, Y H:i"` and the ISO forms for `datetime` attributes), and ratings the built-in `floatformat:1`, because they produce the SPEC's format exactly. Money and counts use two small custom filters (`money`, `count`) built on Python's `,` format specifier, because the built-in `intcomma` and `floatformat:"2g"` follow the locale's grouping settings, which differ by `LANGUAGE_CODE` and `USE_THOUSAND_SEPARATOR`; the store always formats in US style. `USE_THOUSAND_SEPARATOR` stays at its default `False`, so plain integers (page numbers, stock, item counts) render without separators.
- **Star rounding.** The star row's inclusion tag takes the rating in tenths, an integer computed when the catalog is loaded, and fills `(tenths + 5) // 10` stars. Python's `round()` is banker's rounding and would fill 4 stars for 4.5; integer arithmetic avoids that.
- **`USE_I18N = False`.** The store is English only, and the setting's [documentation](https://docs.djangoproject.com/en/6.1/ref/settings/#use-i18n) describes turning it off as a performance optimization. `USE_TZ = True` with `TIME_ZONE = "UTC"`, so order timestamps render in UTC.
- **Error pages.** Unknown paths and unknown product ids raise `Http404`, which Django's default `handler404` renders with `404.html` through a `RequestContext`, so the cart badge works there too. The listing's 422 renders `422.html` with `status=422`.
- **Catalog.** Plain dataclasses with `slots=True`. Related products are resolved to objects at startup, like a prefetched relation; the category of a product is an object, like a foreign key. Formatting, description paragraph splitting, savings, palette lookup and JSON serialization all happen per request. The order summary (revenue, average, refunds) is computed at startup, like an aggregate query.
- **JSON.** `JsonResponse` with Django's default encoder. The product detail and list items are built as dicts per request from the catalog objects. `/api/products/<id>` uses a `str` converter so a non-numeric id gets the JSON 404 instead of the HTML one.
- **Static files.** `collectstatic` with WhiteNoise's `CompressedManifestStaticFilesStorage` writes content-hashed copies (`app.<hash>.css`) plus gzip variants, and WhiteNoise serves the hashed files with `Cache-Control: max-age=315360000, public, immutable`.
