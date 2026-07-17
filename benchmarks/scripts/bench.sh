#!/usr/bin/env bash
# Runs the load benchmark for one or more frameworks and renders a comparison
# table. Usage:
#
#   bench.sh [topcoat|nextjs|leptos|axum_maud ...]     (default: all four)
#
# Tunables (environment variables):
#
#   DURATION=20s WARMUP=5s CONNECTIONS=32 RATE=200 RUNS=3 PORT=8090
#
# Set SINGLE_THREAD=1 to run the Rust servers on a single Tokio worker thread,
# so every framework renders on one core (next start is already single-process).

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

DURATION="${DURATION:-20s}"
WARMUP="${WARMUP:-5s}"
CONNECTIONS="${CONNECTIONS:-32}"
RATE="${RATE:-200}"
RUNS="${RUNS:-3}"

FRAMEWORKS=("$@")
if [ ${#FRAMEWORKS[@]} -eq 0 ]; then
    FRAMEWORKS=(topcoat nextjs leptos axum_maud)
fi

require_cmd oha "brew install oha"
require_cmd jq "brew install jq"
require_port_free

RESULTS_DIR="$BENCH/results/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RESULTS_DIR"

if [ -n "${SINGLE_THREAD:-}" ]; then single_thread_json=true; else single_thread_json=false; fi

cat >"$RESULTS_DIR/meta.json" <<EOF
{
  "git_rev": "$(git -C "$ROOT" rev-parse --short HEAD)",
  "date": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "os": "macOS $(sw_vers -productVersion)",
  "cpu": "$(sysctl -n machdep.cpu.brand_string)",
  "oha": "$(oha --version)",
  "duration": "$DURATION",
  "warmup": "$WARMUP",
  "connections": $CONNECTIONS,
  "rate": $RATE,
  "runs": $RUNS,
  "single_thread": $single_thread_json
}
EOF

for framework in "${FRAMEWORKS[@]}"; do
    echo "==> building $framework"
    "build_$framework"

    echo "==> starting $framework on $BASE"
    LOG_FILE="$RESULTS_DIR/$framework.log"
    "start_$framework"
    trap 'kill_tree "$SERVER_PID"' EXIT INT TERM
    wait_ready

    echo "==> warming up ($WARMUP per route)"
    for path in "${ROUTE_PATHS[@]}"; do
        oha "$BASE$path" -z "$WARMUP" -c "$CONNECTIONS" --no-tui --output-format json >/dev/null
    done

    for run in $(seq 1 "$RUNS"); do
        for i in "${!ROUTE_PATHS[@]}"; do
            label="${ROUTE_LABELS[$i]}"
            path="${ROUTE_PATHS[$i]}"
            echo "==> $framework/$label run $run/$RUNS: throughput ($DURATION, $CONNECTIONS conns)"
            oha "$BASE$path" -z "$DURATION" -c "$CONNECTIONS" --no-tui --output-format json \
                >"$RESULTS_DIR/${framework}_${label}_tput_${run}.json"
            echo "==> $framework/$label run $run/$RUNS: fixed rate ($RATE req/s)"
            oha "$BASE$path" -z "$DURATION" -c "$CONNECTIONS" -q "$RATE" --no-tui --output-format json \
                >"$RESULTS_DIR/${framework}_${label}_rate_${run}.json"
        done
    done

    echo "==> stopping $framework"
    kill_tree "$SERVER_PID"
    trap - EXIT INT TERM
    sleep 1
done

exec "$BENCH/scripts/compare.sh" "$RESULTS_DIR"
