"""Production settings for the Harbor Market storefront.

The benchmark runner configures the app through environment variables: `BENCH_DATA_DIR` points
at the JSON dataset and `BENCH_COMPRESSION=on` enables `GZipMiddleware`. `PORT` and
`BENCH_THREADS` are read by `gunicorn.conf.py`.
"""

import os
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent

# The app has no sessions, logins or signed cookies, but Django requires a key.
SECRET_KEY = os.environ.get("DJANGO_SECRET_KEY", "harbor-market-benchmark-not-secret")

DEBUG = False

ALLOWED_HOSTS = ["127.0.0.1", "localhost"]

# The catalog is loaded from these JSON files at startup; see `store.catalog`.
BENCH_DATA_DIR = Path(os.environ.get("BENCH_DATA_DIR", BASE_DIR.parent.parent / "data"))

COMPRESSION = os.environ.get("BENCH_COMPRESSION", "off")
if COMPRESSION not in ("on", "off"):
    raise ValueError(f"invalid BENCH_COMPRESSION {COMPRESSION!r}")

# There is no database, so the apps that need one (admin, auth, contenttypes, sessions,
# messages) are left out.
INSTALLED_APPS = [
    "django.contrib.staticfiles",
    "store",
]

MIDDLEWARE = [
    "django.middleware.security.SecurityMiddleware",
    "whitenoise.middleware.WhiteNoiseMiddleware",
    *(["django.middleware.gzip.GZipMiddleware"] if COMPRESSION == "on" else []),
    "django.middleware.common.CommonMiddleware",
    "django.middleware.csrf.CsrfViewMiddleware",
    "django.middleware.clickjacking.XFrameOptionsMiddleware",
]

ROOT_URLCONF = "harbor.urls"

# Without an explicit `loaders` option Django wraps the filesystem and app directories loaders
# in the cached loader, so every template is compiled once per process.
TEMPLATES = [
    {
        "BACKEND": "django.template.backends.django.DjangoTemplates",
        "DIRS": [],
        "APP_DIRS": True,
        "OPTIONS": {
            "context_processors": [
                "django.template.context_processors.request",
                "store.context_processors.cart",
            ],
        },
    },
]

WSGI_APPLICATION = "harbor.wsgi.application"

DATABASES = {}

# The store is English only. Turning the translation machinery off is the optimization the
# `USE_I18N` documentation describes.
LANGUAGE_CODE = "en-us"
USE_I18N = False
TIME_ZONE = "UTC"
USE_TZ = True

STATIC_URL = "static/"
STATIC_ROOT = BASE_DIR / "staticfiles"

# WhiteNoise serves the content-hashed files written by `collectstatic` with far-future cache
# headers, plus precompressed gzip variants.
STORAGES = {
    "staticfiles": {
        "BACKEND": "whitenoise.storage.CompressedManifestStaticFilesStorage",
    },
}

# Errors still reach stderr, but nothing is logged per request. Django already sends no request
# logs to the console when `DEBUG` is off; this makes it explicit. Gunicorn's access log is off by
# default.
LOGGING = {
    "version": 1,
    "disable_existing_loggers": False,
    "handlers": {
        "console": {"class": "logging.StreamHandler"},
    },
    "loggers": {
        "django": {"handlers": ["console"], "level": "ERROR", "propagate": False},
    },
}
