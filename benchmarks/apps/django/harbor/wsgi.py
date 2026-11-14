"""WSGI config for the Harbor Market project, served by Gunicorn."""

import os

from django.core.wsgi import get_wsgi_application

os.environ.setdefault("DJANGO_SETTINGS_MODULE", "harbor.settings")

application = get_wsgi_application()
