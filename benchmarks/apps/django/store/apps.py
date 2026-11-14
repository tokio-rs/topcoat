from django.apps import AppConfig


class StoreConfig(AppConfig):
    name = "store"

    def ready(self):
        # Load the dataset and build its indexes at startup. With Gunicorn's `preload_app` this
        # happens once in the master process and the workers share the memory.
        from . import catalog  # noqa: F401
