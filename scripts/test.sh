#!/usr/bin/env bash
# scripts/test.sh — Run all workspace tests with the correct DATABASE_URL for each service.
#
# Usage:
#   ./scripts/test.sh           # run all tests
#   ./scripts/test.sh auth      # run only auth-service tests
#   ./scripts/test.sh url       # run only url-service tests
#   ./scripts/test.sh shared    # run only shared lib tests

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log()    { echo -e "${GREEN}==>${NC} $*"; }
warn()   { echo -e "${YELLOW}==>${NC} $*"; }
error()  { echo -e "${RED}==>${NC} $*"; }

AUTH_DB="postgres://bittu:bittu@localhost:5432/bittuly_auth"
URL_DB="postgres://bittu:bittu@localhost:5433/bittuly_urls"

# ─── Ensure test infrastructure is running ───────────────────────────────────
ensure_infra() {
    log "Ensuring test infrastructure is running..."

    if ! docker ps --format '{{.Names}}' | grep -q "bittuly-postgres-auth"; then
        warn "postgres-auth not running — starting it..."
        docker compose up -d postgres-auth
    fi

    if ! docker ps --format '{{.Names}}' | grep -q "bittuly-postgres-urls"; then
        warn "postgres-urls not running — starting it..."
        docker compose up -d postgres-urls
    fi

    if ! docker ps --format '{{.Names}}' | grep -q "bittuly-redis"; then
        warn "redis not running — starting it..."
        docker compose up -d redis
    fi

    if ! docker ps --format '{{.Names}}' | grep -q "bittuly-rabbitmq"; then
        warn "rabbitmq not running — starting it..."
        docker compose up -d rabbitmq
    fi

    # Wait for postgres-auth
    local retries=15
    until docker exec bittuly-postgres-auth pg_isready -U bittu -d bittuly_auth -q 2>/dev/null; do
        retries=$((retries - 1))
        if [ $retries -le 0 ]; then error "postgres-auth never became ready"; exit 1; fi
        sleep 1
    done

    # Wait for postgres-urls
    retries=15
    until docker exec bittuly-postgres-urls pg_isready -U bittu -d bittuly_urls -q 2>/dev/null; do
        retries=$((retries - 1))
        if [ $retries -le 0 ]; then error "postgres-urls never became ready"; exit 1; fi
        sleep 1
    done

    log "Infrastructure is ready."
}

run_shared() {
    log "Testing shared lib..."
    cargo test -p shared 2>&1
}

run_auth() {
    log "Testing auth-service (DATABASE_URL → postgres-auth :5432)..."
    DATABASE_URL="$AUTH_DB" cargo test -p auth-service 2>&1
}

run_url() {
    log "Testing url-service (DATABASE_URL → postgres-urls :5433)..."
    DATABASE_URL="$URL_DB" cargo test -p url-service 2>&1
}

# ─── Main ────────────────────────────────────────────────────────────────────
ensure_infra

TARGET="${1:-all}"

case "$TARGET" in
    auth)   run_auth ;;
    url)    run_url ;;
    shared) run_shared ;;
    all)
        run_shared
        run_auth
        run_url
        log "All tests passed! ✅"
        ;;
    *)
        error "Unknown target: $TARGET. Use: auth | url | shared | all"
        exit 1
        ;;
esac
