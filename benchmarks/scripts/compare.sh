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

    # Column headers and per-column alignment (l = left, r = right). Data rows
    # are buffered so the columns can be padded to a common width; the raw
    # markdown then lines up when read as plain text, and still renders in any
    # markdown viewer.
    headers=("Route" "Framework" "req/s (median)" "vs topcoat" "p50 ms" "p90 ms" "p99 ms" "bytes/resp" "success")
    aligns=(l l r r r r r r r)
    rows=()

    for i in "${!ROUTE_LABELS[@]}"; do
        label="${ROUTE_LABELS[$i]}"
        path="${ROUTE_PATHS[$i]}"
        base_rps=""
        for framework in topcoat nextjs leptos axum_maud; do
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

            # Throughput relative to topcoat, the framework under test. Topcoat
            # is the baseline; the others report how many times faster or slower
            # they served this route (by req/s). "n/a" when topcoat is absent.
            if [ "$framework" = topcoat ]; then
                base_rps="$rps"
                rel="baseline"
            elif [ -n "$base_rps" ] && [ "$base_rps" -gt 0 ]; then
                rel=$(awk -v a="$rps" -v b="$base_rps" 'BEGIN {
                    if (a >= b) printf "%.2fx faster", a / b
                    else printf "%.2fx slower", b / a
                }')
            else
                rel="n/a"
            fi

            rows+=("$(printf '`%s`\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s' \
                "$path" "$framework" "$rps" "$rel" "$p50" "$p90" "$p99" "$size" "$success")")
        done
    done

    # Column widths: the widest of the header and any cell in that column.
    ncol=${#headers[@]}
    widths=()
    for c in $(seq 0 $((ncol - 1))); do widths[c]=${#headers[c]}; done
    for row in "${rows[@]}"; do
        IFS=$'\t' read -r -a cells <<<"$row"
        for c in $(seq 0 $((ncol - 1))); do
            (( ${#cells[c]} > widths[c] )) && widths[c]=${#cells[c]}
        done
    done

    # Pad one cell to its column width, honoring alignment.
    cell() {
        if [ "${aligns[$2]}" = r ]; then printf '%*s' "${widths[$2]}" "$1"
        else printf '%-*s' "${widths[$2]}" "$1"; fi
    }

    # Header row.
    line="|"
    for c in $(seq 0 $((ncol - 1))); do line+=" $(cell "${headers[c]}" "$c") |"; done
    echo "$line"

    # Separator row: dashes to the column width, with a trailing colon marking
    # right-aligned columns.
    line="|"
    for c in $(seq 0 $((ncol - 1))); do
        dashes=$(printf '%*s' "${widths[c]}" ''); dashes=${dashes// /-}
        [ "${aligns[c]}" = r ] && dashes="${dashes%?}:"
        line+=" $dashes |"
    done
    echo "$line"

    # Data rows.
    for row in "${rows[@]}"; do
        IFS=$'\t' read -r -a cells <<<"$row"
        line="|"
        for c in $(seq 0 $((ncol - 1))); do line+=" $(cell "${cells[c]}" "$c") |"; done
        echo "$line"
    done
} | tee "$SUMMARY"

echo
echo "Written to $SUMMARY"
