"""Gunicorn settings, read from the benchmark environment.

`BENCH_THREADS` unset or `0` starts one worker process per core; `N` starts `N` workers.
"""

import os

bind = f"127.0.0.1:{os.environ.get('PORT', '8000')}"

# The gthread worker keeps client connections alive; the default sync worker closes every
# connection after one response because it expects a buffering proxy such as nginx in front.
# One request thread per process: the work is CPU-bound Python, so more threads would only
# contend for the GIL, and scaling comes from the processes.
worker_class = "gthread"
threads = 1
workers = int(os.environ.get("BENCH_THREADS") or 0) or os.process_cpu_count()

# Load Django and the catalog once in the master so workers share it copy-on-write.
preload_app = True

# No access log (Gunicorn's default); startup messages and errors go to stderr.
accesslog = None
errorlog = "-"
loglevel = "warning"
