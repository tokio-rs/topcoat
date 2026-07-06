#!/usr/bin/env bash
# Shared helpers for the benchmark scripts. Source this file, do not run it.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BENCH="$ROOT/benchmarks"

PORT="${PORT:-8090}"
BASE="http://127.0.0.1:${PORT}"

# When SINGLE_THREAD is set, the Rust servers (Topcoat, Leptos) run with a
# single Tokio worker thread, so their request handling is single-threaded like
# next start's one Node process. Next.js is already single-process and is not
# affected. Empty otherwise, so the Rust servers use every core.
if [ -n "${SINGLE_THREAD:-}" ]; then
    RUST_SERVER_ENV="TOKIO_WORKER_THREADS=1"
else
    RUST_SERVER_ENV=""
fi

# The routes every framework is measured on. Labels are used in file names.
ROUTE_LABELS=(home products product)
ROUTE_PATHS=("/" "/products?page=3&sort=price" "/products/42")

# Set by the start_* helpers; used by kill_tree.
SERVER_PID=""
# Server output is redirected here; callers set it before start_*.
LOG_FILE="${LOG_FILE:-/dev/null}"

require_cmd() {
    local cmd="$1" hint="$2"
    if ! command -v "$cmd" >/dev/null 2>&1; then
        echo "error: $cmd is not installed (install with: $hint)" >&2
        exit 1
    fi
}

require_port_free() {
    if lsof -ti "tcp:${PORT}" >/dev/null 2>&1; then
        echo "error: port ${PORT} is already in use" >&2
        exit 1
    fi
}

wait_ready() {
    for _ in $(seq 1 240); do
        if curl -sf -o /dev/null "$BASE/"; then
            return 0
        fi
        sleep 0.25
    done
    echo "error: server did not become ready on $BASE" >&2
    return 1
}

# Terminates a server process and its children (pnpm/next spawn workers), then
# makes sure nothing is left holding the port.
kill_tree() {
    local pid="$1"
    [ -n "$pid" ] || return 0
    pkill -TERM -P "$pid" 2>/dev/null || true
    kill -TERM "$pid" 2>/dev/null || true
    for _ in $(seq 1 20); do
        if ! kill -0 "$pid" 2>/dev/null && ! lsof -ti "tcp:${PORT}" >/dev/null 2>&1; then
            return 0
        fi
        sleep 0.25
    done
    pkill -KILL -P "$pid" 2>/dev/null || true
    kill -KILL "$pid" 2>/dev/null || true
    lsof -ti "tcp:${PORT}" 2>/dev/null | xargs kill -KILL 2>/dev/null || true
}

build_topcoat() {
    (cd "$ROOT" && cargo build --release -p storefront-topcoat)
    (cd "$ROOT" && topcoat asset bundle --package storefront-topcoat)
}

start_topcoat() {
    env $RUST_SERVER_ENV PORT="$PORT" "$ROOT/target/release/storefront-topcoat" >"$LOG_FILE" 2>&1 &
    SERVER_PID=$!
}

build_nextjs() {
    (cd "$BENCH/nextjs" && pnpm install --frozen-lockfile)
    (cd "$BENCH/nextjs" && NEXT_TELEMETRY_DISABLED=1 pnpm build)
}

start_nextjs() {
    (
        cd "$BENCH/nextjs" &&
            NEXT_TELEMETRY_DISABLED=1 exec node_modules/.bin/next start -H 127.0.0.1 -p "$PORT"
    ) >"$LOG_FILE" 2>&1 &
    SERVER_PID=$!
}

build_leptos() {
    (cd "$BENCH/leptos" && LEPTOS_TAILWIND_VERSION=v4.3.2 cargo leptos build --release)
}

start_leptos() {
    (
        cd "$BENCH/leptos" &&
            LEPTOS_SITE_ADDR="127.0.0.1:${PORT}" LEPTOS_SITE_ROOT=target/site \
                exec env $RUST_SERVER_ENV target/release/storefront-leptos
    ) >"$LOG_FILE" 2>&1 &
    SERVER_PID=$!
}
