#!/usr/bin/env bash
# =============================================================================
# api-test.sh  —  API flow test for Bittuly (OTP signup edition)
#
# Requires: curl, bash ≥4
# Optional: jq (for pretty-printing and OTP extraction)
#
# Usage:
#   ./scripts/api-test.sh
#   BASE_URL=http://my-server:3000 ./scripts/api-test.sh
#
# NOTE: Server must be running with MODE=development so /debug/otp-store
#       is available and no real emails are sent.
#
# CLEANUP: URLs have no DELETE endpoint yet — deleting the user is the only
#          cleanup available. Add DELETE /{short_code} to clean up URLs too.
# =============================================================================
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:3000}"
STAMP="$(date +%s)"
EMAIL="bittu-${STAMP}@example.com"
USERNAME="bittu-${STAMP}"
PASSWORD="secret123"
UPDATED_EMAIL="bittu-updated-${STAMP}@example.com"
UPDATED_USERNAME="bittu-updated-${STAMP}"
UPDATED_PASSWORD="secret-updated"

# ── colours ───────────────────────────────────────────────────────────────────
BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; MAGENTA='\033[0;35m'

# ── global state ──────────────────────────────────────────────────────────────
RESPONSE_BODY=""
RESPONSE_HEADERS=""
COOKIE_JAR=""

# ── helpers ───────────────────────────────────────────────────────────────────
section() {
  local width=62
  local line; line="$(printf '─%.0s' $(seq 1 $width))"
  printf "\n${BOLD}${BLUE}┌%s┐${NC}\n" "$line"
  printf   "${BOLD}${BLUE}│  %-60s│${NC}\n" "$*"
  printf   "${BOLD}${BLUE}└%s┘${NC}\n" "$line"
}

kv() {
  local c="${3:-$NC}"
  printf "    ${DIM}%-18s${NC}${c}%s${NC}\n" "$1:" "$2"
}

pretty_json() {
  if command -v jq &>/dev/null; then
    printf '%s' "$1" | jq . 2>/dev/null && return
  fi
  printf '%s\n' "$1"
}

cookie_get() {
  printf '%s' "$COOKIE_JAR" \
    | tr ';' '\n' \
    | grep -oP "^\s*$1=\K.*" \
    | head -1 \
    | tr -d ' ' \
    || true
}

update_cookie_jar() {
  while IFS= read -r line; do
    if [[ "$line" =~ ^[Ss]et-[Cc]ookie:[[:space:]]*([A-Za-z_]+)=([^;]*) ]]; then
      local name="${BASH_REMATCH[1]}"
      local value="${BASH_REMATCH[2]}"
      if printf '%s' "$line" | grep -qi "Max-Age=0"; then
        COOKIE_JAR="$(printf '%s' "$COOKIE_JAR" \
          | tr ';' '\n' \
          | grep -v "^\s*${name}=" \
          | tr '\n' ';' \
          | sed 's/^;//;s/;$//')"
      else
        if printf '%s' "$COOKIE_JAR" | grep -q "${name}="; then
          COOKIE_JAR="$(printf '%s' "$COOKIE_JAR" \
            | tr ';' '\n' \
            | sed "s|^\s*${name}=.*|${name}=${value}|" \
            | tr '\n' ';' \
            | sed 's/^;//;s/;$//')"
        else
          COOKIE_JAR="${COOKIE_JAR:+${COOKIE_JAR}; }${name}=${value}"
        fi
      fi
    fi
  done <<< "$RESPONSE_HEADERS"
}

do_request() {
  local method="$1" url="$2"
  shift 2

  local -a curl_args=()
  local -a req_headers=()
  local req_body=""

  if [[ -n "$COOKIE_JAR" ]]; then
    req_headers+=("Cookie: $COOKIE_JAR")
    curl_args+=(-H "Cookie: $COOKIE_JAR")
  fi

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -H) req_headers+=("$2"); curl_args+=(-H "$2"); shift 2 ;;
      -d) req_body="$2";       curl_args+=(-d "$2"); shift 2 ;;
      *)  curl_args+=("$1");   shift ;;
    esac
  done

  printf "\n${CYAN}${BOLD}  ▶  REQUEST${NC}\n"
  kv "Method" "$method" "${BOLD}"
  kv "URL"    "$url"    "${BOLD}"
  if [[ ${#req_headers[@]} -gt 0 ]]; then
    printf "    ${DIM}%-18s${NC}\n" "Headers"
    for h in "${req_headers[@]}"; do
      printf "      ${YELLOW}%s${NC}\n" "$h"
    done
  fi
  if [[ -n "$req_body" ]]; then
    printf "    ${DIM}%-18s${NC}\n" "Body"
    pretty_json "$req_body" | sed 's/^/      /'
  fi

  local raw
  raw="$(curl -sS -i -X "$method" "$url" "${curl_args[@]}")"

  local status_line
  status_line="$(printf '%s' "$raw" | head -1 | tr -d '\r')"
  RESPONSE_HEADERS="$(printf '%s' "$raw" \
    | awk 'NR==1{next} /^\r?$/{exit} {sub(/\r/,""); print}')"
  RESPONSE_BODY="$(printf '%s' "$raw" \
    | awk 'BEGIN{p=0} /^\r?$/{if(!p){p=1;next}} p{print}')"

  update_cookie_jar

  local status_code
  status_code="$(printf '%s' "$status_line" | grep -oE '[0-9]{3}' | head -1)"
  local sc_color="$GREEN"
  [[ "$status_code" =~ ^3 ]] && sc_color="$YELLOW"
  [[ "$status_code" =~ ^4 ]] && sc_color="$RED"
  [[ "$status_code" =~ ^5 ]] && sc_color="$RED"

  printf "\n${GREEN}${BOLD}  ◀  RESPONSE${NC}\n"
  kv "Status" "$status_line" "${BOLD}${sc_color}"

  if [[ -n "$RESPONSE_HEADERS" ]]; then
    printf "    ${DIM}%-18s${NC}\n" "Headers"
    while IFS= read -r line; do
      printf "      ${YELLOW}%s${NC}\n" "$line"
    done <<< "$RESPONSE_HEADERS"
  fi

  if [[ -n "$RESPONSE_BODY" ]]; then
    printf "    ${DIM}%-18s${NC}\n" "Body"
    if command -v jq &>/dev/null \
        && printf '%s' "$RESPONSE_BODY" | jq . &>/dev/null 2>&1; then
      printf '%s' "$RESPONSE_BODY" | jq . | sed 's/^/      /'
    else
      printf "      %s\n" "$RESPONSE_BODY"
    fi
  fi

  if [[ -n "$COOKIE_JAR" ]]; then
    printf "\n    ${MAGENTA}${BOLD}Cookie jar:${NC}${MAGENTA} %s${NC}\n" \
      "$(printf '%s' "$COOKIE_JAR" | cut -c1-80)"
  fi
}

extract() {
  printf '%s' "$2" | grep -oP "\"$1\":\s*\"\K[^\"]+" 2>/dev/null || true
}

extract_otp_for_email() {
  local email="$1" body="$2"
  if command -v jq &>/dev/null; then
    printf '%s' "$body" \
      | jq -r --arg e "$email" '.[] | select(.email == $e) | .otp' \
      2>/dev/null | head -1 || true
  else
    printf '%s' "$body" | grep -oP '"otp":\s*"\K[0-9]+' | head -1 || true
  fi
}

captured() {
  printf "\n    ${MAGENTA}${BOLD}Captured:${NC}"
  while [[ $# -ge 2 ]]; do
    printf "${MAGENTA}  %s=%s${NC}" "$1" "$2"; shift 2
  done
  printf "\n"
}

# Assert a cookie is present in the jar. Exits with error if missing.
check_cookie() {
  local name="$1" context="$2"
  local val; val="$(cookie_get "$name")"
  if [[ -n "$val" ]]; then
    printf "    ${GREEN}${BOLD}✓ cookie${NC}  ${name} is set after %s\n" "$context"
  else
    printf "    ${RED}${BOLD}✗ cookie${NC}  ${name} missing after %s — aborting\n" "$context" >&2
    exit 1
  fi
}

# Assert a cookie has been cleared from the jar. Exits with error if still present.
check_cookie_cleared() {
  local name="$1" context="$2"
  local val; val="$(cookie_get "$name")"
  if [[ -z "$val" ]]; then
    printf "    ${GREEN}${BOLD}✓ cookie${NC}  ${name} cleared after %s\n" "$context"
  else
    printf "    ${RED}${BOLD}✗ cookie${NC}  ${name} still set after %s — aborting\n" "$context" >&2
    exit 1
  fi
}

# ── connectivity guard ─────────────────────────────────────────────────────────
printf "\n${BOLD}Checking server at ${BASE_URL}...${NC}\n"
HTTP_CODE="$(curl -so /dev/null --max-time 3 -w '%{http_code}' "$BASE_URL" 2>/dev/null || true)"
if [[ "$HTTP_CODE" == "000" ]]; then
  printf "${RED}${BOLD}ERROR: Server is not reachable at ${BASE_URL}${NC}\n"
  printf "${YELLOW}Start the server first: cargo run${NC}\n\n"
  exit 1
fi
printf "${GREEN}${BOLD}Server is up.${NC}\n"

# =============================================================================
section "1 · Signup — POST /users/signup  (request OTP)"
do_request POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${USERNAME}\",\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}"

PENDING_TOKEN="$(extract pending_token "$RESPONSE_BODY")"
[[ -z "$PENDING_TOKEN" ]] && { printf "${RED}ERROR: no pending_token${NC}\n" >&2; exit 1; }
captured "pending_token" "${PENDING_TOKEN:0:40}..."

# =============================================================================
section "2 · Fetch OTP — GET /debug/otp-store"
do_request GET "$BASE_URL/debug/otp-store"

OTP="$(extract_otp_for_email "$EMAIL" "$RESPONSE_BODY")"
[[ -z "$OTP" ]] && {
  printf "${RED}ERROR: OTP not found for ${EMAIL}. Is MODE=development?${NC}\n" >&2; exit 1
}
captured "otp" "$OTP"

# =============================================================================
section "3 · Verify OTP — POST /users/verify-otp  (create user + JWT)"
do_request POST "$BASE_URL/users/verify-otp" \
  -H "Content-Type: application/json" \
  -d "{\"pending_token\":\"${PENDING_TOKEN}\",\"otp\":\"${OTP}\"}"

USER_ID="$(extract id "$RESPONSE_BODY")"
[[ -z "$USER_ID" ]] && { printf "${RED}ERROR: no user id${NC}\n" >&2; exit 1; }
captured "user_id" "$USER_ID"

# Cookies must be set after successful signup
check_cookie "access_token"  "verify-otp"
check_cookie "refresh_token" "verify-otp"

# =============================================================================
section "4 · Login — POST /users/login"
COOKIE_JAR=""  # clear jar to simulate a fresh login session
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}"

check_cookie "access_token"  "login"
check_cookie "refresh_token" "login"

# =============================================================================
section "5 · Get user — GET /users/$USER_ID"
do_request GET "$BASE_URL/users/$USER_ID"

# =============================================================================
section "6 · Update user — PUT /users/$USER_ID"
do_request PUT "$BASE_URL/users/$USER_ID" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${UPDATED_USERNAME}\",\"email\":\"${UPDATED_EMAIL}\",\"password\":\"${UPDATED_PASSWORD}\"}"

# Cookies are rotated on update — verify they are still present
check_cookie "access_token"  "update-user"
check_cookie "refresh_token" "update-user"

# =============================================================================
section "7 · Shorten URL — POST /"
do_request POST "$BASE_URL/" \
  -H "Content-Type: application/json" \
  -d '{"original_url":"https://example.com"}'

SHORT_CODE="$(extract short_code "$RESPONSE_BODY")"
[[ -z "$SHORT_CODE" ]] && { printf "${RED}ERROR: no short_code${NC}\n" >&2; exit 1; }
captured "short_code" "$SHORT_CODE"

# =============================================================================
section "8 · Get all URLs — GET /"
do_request GET "$BASE_URL/"

# =============================================================================
section "9 · Resolve short URL — GET /$SHORT_CODE  (public, no auth)"
SAVED_JAR="$COOKIE_JAR"
COOKIE_JAR=""                          # public endpoint — no cookies sent
do_request GET "$BASE_URL/$SHORT_CODE"
COOKIE_JAR="$SAVED_JAR"               # restore jar for subsequent protected steps

# =============================================================================
section "10 · Logout — POST /users/logout"
do_request POST "$BASE_URL/users/logout"

# Cookies must be cleared after logout
check_cookie_cleared "access_token"  "logout"
check_cookie_cleared "refresh_token" "logout"
printf "    ${MAGENTA}Cookie jar: '%s'${NC}\n" "$COOKIE_JAR"

# =============================================================================
# ── Cleanup ──────────────────────────────────────────────────────────────────
# NOTE: No DELETE /urls/:id endpoint exists yet — URLs created during this
#       run cannot be deleted via the API. Deleting the user below is the
#       only available cleanup. Add a URL delete route to clean up URLs too.
# =============================================================================
section "11 · Re-login for cleanup — POST /users/login"
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${UPDATED_EMAIL}\",\"password\":\"${UPDATED_PASSWORD}\"}"

check_cookie "access_token"  "re-login"
check_cookie "refresh_token" "re-login"

# =============================================================================
section "12 · Delete user — DELETE /users/$USER_ID  (cleanup)"
do_request DELETE "$BASE_URL/users/$USER_ID"

printf "\n${BOLD}${GREEN}✓ All steps completed. User ${USER_ID} deleted.${NC}\n"
printf "${YELLOW}⚠  URLs created during this run were not deleted (no delete endpoint).${NC}\n\n"
