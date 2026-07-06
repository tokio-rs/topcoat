#!/usr/bin/env bash
# Verifies that every framework renders the same visible text. Builds and
# starts each framework in turn, fetches a set of routes, reduces the HTML to
# normalized text, and diffs everything against the Topcoat rendering. Usage:
#
#   verify_parity.sh [topcoat|nextjs|leptos ...]     (default: all three)

set -euo pipefail
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/common.sh"

# More routes than the benchmark itself: also cover the last page (disabled
# "Next" link), a category filter combined with a sort, and the home page.
PARITY_LABELS=(home products product last_page category)
PARITY_PATHS=(
    "/"
    "/products?page=3&sort=price"
    "/products/42"
    "/products?page=21"
    "/products?category=audio&sort=rating"
)

FRAMEWORKS=("$@")
if [ ${#FRAMEWORKS[@]} -eq 0 ]; then
    FRAMEWORKS=(topcoat nextjs leptos)
fi

require_cmd node "brew install node"
require_port_free

PARITY_DIR="$BENCH/results/parity"
mkdir -p "$PARITY_DIR"

for framework in "${FRAMEWORKS[@]}"; do
    echo "==> building $framework"
    "build_$framework"

    echo "==> starting $framework"
    LOG_FILE="$PARITY_DIR/$framework.log"
    "start_$framework"
    trap 'kill_tree "$SERVER_PID"' EXIT INT TERM
    wait_ready

    for i in "${!PARITY_PATHS[@]}"; do
        label="${PARITY_LABELS[$i]}"
        path="${PARITY_PATHS[$i]}"
        curl -s "$BASE$path" >"$PARITY_DIR/${framework}_${label}.html"
        node "$BENCH/scripts/extract_text.mjs" \
            <"$PARITY_DIR/${framework}_${label}.html" \
            >"$PARITY_DIR/${framework}_${label}.txt"
    done

    if [ "$framework" = "nextjs" ]; then
        if ! curl -sD - -o /dev/null "$BASE/products/42" | grep -qi "cache-control: .*no-store"; then
            echo "error: nextjs response is missing no-store; force-dynamic is not active" >&2
            kill_tree "$SERVER_PID"
            exit 1
        fi
    fi

    echo "==> stopping $framework"
    kill_tree "$SERVER_PID"
    trap - EXIT INT TERM
    sleep 1
done

status=0
for framework in "${FRAMEWORKS[@]}"; do
    [ "$framework" = "topcoat" ] && continue
    for label in "${PARITY_LABELS[@]}"; do
        reference="$PARITY_DIR/topcoat_${label}.txt"
        candidate="$PARITY_DIR/${framework}_${label}.txt"
        if [ ! -f "$reference" ] || [ ! -f "$candidate" ]; then
            echo "skip: $label (missing rendering for topcoat or $framework)"
            continue
        fi
        if diff -q "$reference" "$candidate" >/dev/null; then
            echo "ok:   $framework/$label matches topcoat"
        else
            echo "FAIL: $framework/$label differs from topcoat:"
            diff "$reference" "$candidate" | head -10 || true
            status=1
        fi
    done
done

exit "$status"
