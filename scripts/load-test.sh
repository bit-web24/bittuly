#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# load-test.sh — Run k6 load tests against the local cluster using Docker
#
# Usage:
#   ./scripts/load-test.sh redirects   # Runs tests/load/redirects.js
#   ./scripts/load-test.sh shorten     # Runs tests/load/shorten.js
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

log()   { echo -e "\033[0;32m==>\033[0m $*"; }
error() { echo -e "\033[0;31m==>\033[0m $*" >&2; }

if [ "$#" -lt 1 ]; then
    error "Usage: ./scripts/load-test.sh <script-name>"
    echo "Available scripts:"
    ls -1 tests/load/*.js | xargs -n 1 basename | sed 's/\.js$//' | sed 's/^/  - /'
    exit 1
fi

SCRIPT_NAME="$1"
SCRIPT_PATH="tests/load/${SCRIPT_NAME}.js"

if [ ! -f "$SCRIPT_PATH" ]; then
    error "Script not found: $SCRIPT_PATH"
    exit 1
fi

log "Running load test: $SCRIPT_NAME"
log "Target: http://localhost:8000"

# We use --network host so the container can reach localhost:8000 on the Linux host
docker run --rm \
    --network host \
    -i grafana/k6 run \
    -e BASE_URL="http://localhost:8000" \
    - < "$SCRIPT_PATH"
