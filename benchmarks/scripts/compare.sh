#!/usr/bin/env bash
# Renders a markdown comparison table from a results directory produced by
# bench.sh. Usage:
#
#   compare.sh [results-dir]     (default: the newest directory under results/)

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

require_cmd jq "brew install jq"

RESULTS_DIR="${1:-$(find "$BENCH/results" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort | tail -1)}"
RESULTS_DIR="${RESULTS_DIR%/}"
if [ -z "$RESULTS_DIR" ] || [ ! -d "$RESULTS_DIR" ]; then
    echo "error: no results directory found" >&2
    exit 1
fi

SUMMARY="$RESULTS_DIR/summary.md"

# Median of a jq expression across a set of run files.
median() {
    local expr="$1"
    shift
    jq -s "[.[] | $expr] | sort | .[length / 2 | floor]" "$@"
}

{
    echo "# Benchmark results"
    echo
    echo "Source: \`$RESULTS_DIR\`"
    jq -r '"Machine: \(.cpu), \(.os) | oha \(.oha) | \(.runs) runs x \(.duration) at \(.connections) connections, fixed rate \(.rate) req/s\(if .single_thread then " | single-threaded (TOKIO_WORKER_THREADS=1)" else "" end)"' \
        "$RESULTS_DIR/meta.json" 2>/dev/null || true
    echo
    echo "| Route | Framework | req/s (median) | p50 ms | p90 ms | p99 ms | bytes/resp | success |"
    echo "|-------|-----------|----------------|--------|--------|--------|------------|---------|"

    for i in "${!ROUTE_LABELS[@]}"; do
        label="${ROUTE_LABELS[$i]}"
        path="${ROUTE_PATHS[$i]}"
        for framework in topcoat nextjs leptos; do
            tput_files=("$RESULTS_DIR/${framework}_${label}_tput_"*.json)
            rate_files=("$RESULTS_DIR/${framework}_${label}_rate_"*.json)
            [ -f "${tput_files[0]}" ] || continue

            rps=$(median '.summary.requestsPerSec | round' "${tput_files[@]}")
            p50=$(median '.latencyPercentiles.p50 * 100000 | round / 100' "${rate_files[@]}")
            p90=$(median '.latencyPercentiles.p90 * 100000 | round / 100' "${rate_files[@]}")
            p99=$(median '.latencyPercentiles.p99 * 100000 | round / 100' "${rate_files[@]}")
            size=$(jq -s '[.[].summary.sizePerRequest] | add / length | round' "${rate_files[@]}")
            success=$(jq -s '[.[].summary.successRate] | min' "${tput_files[@]}" "${rate_files[@]}")

            codes=$(jq -s '[.[].statusCodeDistribution | keys[]] | unique | join(",")' \
                "${tput_files[@]}" "${rate_files[@]}")
            if [ "$codes" != '"200"' ]; then
                echo "warning: $framework/$label saw status codes $codes" >&2
            fi

            echo "| \`$path\` | $framework | $rps | $p50 | $p90 | $p99 | $size | $success |"
        done
    done
} | tee "$SUMMARY"

echo
echo "Written to $SUMMARY"
