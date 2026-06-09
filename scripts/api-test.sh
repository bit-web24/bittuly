#!/usr/bin/env bash
# =============================================================================
# api-test.sh  —  Full API flow test for Bittuly (OTP signup edition)
#
# Requires: curl, bash ≥4
# Optional: jq (for pretty-printing and OTP extraction from array responses)
#
# Usage:
#   ./scripts/api-test.sh
#   BASE_URL=http://my-server:3000 ./scripts/api-test.sh
#
# NOTE: Server must be running with MODE=development so /debug/otp-store
#       is available and no real emails are sent.
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
RESPONSE_STATUS=""
COOKIE_JAR=""
PASS=0
FAIL=0

# ── helpers ───────────────────────────────────────────────────────────────────
section() {
  local width=62
  local line; line="$(printf '─%.0s' $(seq 1 $width))"
  printf "\n${BOLD}${BLUE}┌%s┐${NC}\n" "$line"
  printf   "${BOLD}${BLUE}│  %-60s│${NC}\n" "$*"
  printf   "${BOLD}${BLUE}└%s┘${NC}\n" "$line"
}

edge_case() {
  local width=62
  local line; line="$(printf '╌%.0s' $(seq 1 $width))"
  printf "\n${BOLD}${YELLOW}┌%s┐${NC}\n" "$line"
  printf   "${BOLD}${YELLOW}│  %-60s│${NC}\n" "⚠  EDGE CASE: $*"
  printf   "${BOLD}${YELLOW}└%s┘${NC}\n" "$line"
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

  RESPONSE_STATUS="$(printf '%s' "$status_line" | grep -oE '[0-9]{3}' | head -1)"
  local sc_color="$GREEN"
  [[ "$RESPONSE_STATUS" =~ ^3 ]] && sc_color="$YELLOW"
  [[ "$RESPONSE_STATUS" =~ ^4 ]] && sc_color="$RED"
  [[ "$RESPONSE_STATUS" =~ ^5 ]] && sc_color="$RED"

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

# Extract a scalar value from flat JSON.
extract() {
  printf '%s' "$2" | grep -oP "\"$1\":\s*\"\K[^\"]+" 2>/dev/null || true
}

# Extract a value from a JSON array's first element matching a given email.
# Falls back to the first element if jq is unavailable.
extract_otp_for_email() {
  local email="$1" body="$2"
  if command -v jq &>/dev/null; then
    printf '%s' "$body" \
      | jq -r --arg e "$email" '.[] | select(.email == $e) | .otp' \
      2>/dev/null | head -1 || true
  else
    # Fallback: grab the first otp value in the response
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

# Print PASS / FAIL for a status assertion (does NOT exit on failure).
check_status() {
  local expected="$1" label="$2"
  if [[ "$RESPONSE_STATUS" == "$expected" ]]; then
    printf "    ${GREEN}${BOLD}✓ PASS${NC}  %-42s → %s\n" "$label" "$RESPONSE_STATUS"
    PASS=$((PASS + 1))
  else
    printf "    ${RED}${BOLD}✗ FAIL${NC}  %-42s → expected ${expected}, got ${RESPONSE_STATUS}\n" "$label"
    FAIL=$((FAIL + 1))
  fi
}

# ── connectivity guard ─────────────────────────────────────────────────────────
printf "\n${BOLD}Checking server at ${BASE_URL}...${NC}\n"
if ! curl -sf --max-time 3 "$BASE_URL" &>/dev/null; then
  # 404 is fine — it means the server is up
  HTTP_CODE="$(curl -so /dev/null --max-time 3 -w '%{http_code}' "$BASE_URL" 2>/dev/null || true)"
  if [[ "$HTTP_CODE" == "000" ]]; then
    printf "${RED}${BOLD}ERROR: Server is not reachable at ${BASE_URL}${NC}\n"
    printf "${YELLOW}Start the server first: cargo run${NC}\n\n"
    exit 1
  fi
fi
printf "${GREEN}${BOLD}Server is up.${NC}\n"

# =============================================================================
edge_case "Signup — invalid email format  →  expect 422"
do_request POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d '{"username":"validuser","email":"not-an-email","password":"secret123"}'
check_status "422" "Invalid email → 422"

# =============================================================================
edge_case "Signup — password too short (< 6 chars)  →  expect 422"
do_request POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"validuser\",\"email\":\"${EMAIL}\",\"password\":\"abc\"}"
check_status "422" "Short password → 422"

# =============================================================================
edge_case "Signup — username too short (< 3 chars)  →  expect 422"
do_request POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"ab\",\"email\":\"${EMAIL}\",\"password\":\"secret123\"}"
check_status "422" "Short username → 422"

# =============================================================================
edge_case "Signup — missing required fields  →  expect 422"
do_request POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d '{"username":"validuser"}'
check_status "422" "Missing fields → 422"

# =============================================================================
section "1 · Signup Step 1 — POST /users/signup  (send OTP)"
do_request POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${USERNAME}\",\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}"
check_status "200" "Signup request → 200"

PENDING_TOKEN="$(extract pending_token "$RESPONSE_BODY")"
[[ -z "$PENDING_TOKEN" ]] && {
  printf "${RED}${BOLD}ERROR: no pending_token in response${NC}\n" >&2; exit 1
}
captured "pending_token" "${PENDING_TOKEN:0:40}..."

# =============================================================================
section "2 · Fetch OTP — GET /debug/otp-store"
do_request GET "$BASE_URL/debug/otp-store"
check_status "200" "Debug OTP store → 200"

OTP="$(extract_otp_for_email "$EMAIL" "$RESPONSE_BODY")"
[[ -z "$OTP" ]] && {
  printf "${RED}${BOLD}ERROR: OTP not found in debug store for ${EMAIL}${NC}\n" >&2
  printf "${YELLOW}Is the server running with MODE=development?${NC}\n\n" >&2
  exit 1
}
captured "otp" "$OTP" "email" "$EMAIL"

# =============================================================================
edge_case "Verify OTP — wrong OTP submitted  →  expect 401"
do_request POST "$BASE_URL/users/verify-otp" \
  -H "Content-Type: application/json" \
  -d "{\"pending_token\":\"${PENDING_TOKEN}\",\"otp\":\"000000\"}"
check_status "401" "Wrong OTP → 401"

# =============================================================================
edge_case "Verify OTP — otp field wrong length  →  expect 422"
do_request POST "$BASE_URL/users/verify-otp" \
  -H "Content-Type: application/json" \
  -d "{\"pending_token\":\"${PENDING_TOKEN}\",\"otp\":\"123\"}"
check_status "422" "OTP wrong length → 422"

# =============================================================================
edge_case "Verify OTP — invalid/tampered pending_token  →  expect 5xx"
do_request POST "$BASE_URL/users/verify-otp" \
  -H "Content-Type: application/json" \
  -d "{\"pending_token\":\"this.is.not.a.valid.jwt\",\"otp\":\"${OTP}\"}"
# Server will fail to decode the JWT → 500
[[ "$RESPONSE_STATUS" =~ ^[45] ]] && {
  printf "    ${GREEN}${BOLD}✓ PASS${NC}  Tampered token rejected → %s\n" "$RESPONSE_STATUS"
  PASS=$((PASS + 1))
} || {
  printf "    ${RED}${BOLD}✗ FAIL${NC}  Expected 4xx/5xx, got %s\n" "$RESPONSE_STATUS"
  FAIL=$((FAIL + 1))
}

# =============================================================================
section "3 · Signup Step 2 — POST /users/verify-otp  (create user + JWT)"
do_request POST "$BASE_URL/users/verify-otp" \
  -H "Content-Type: application/json" \
  -d "{\"pending_token\":\"${PENDING_TOKEN}\",\"otp\":\"${OTP}\"}"
check_status "201" "Verify OTP → 201"

USER_ID="$(extract id "$RESPONSE_BODY")"
[[ -z "$USER_ID" ]] && {
  printf "${RED}${BOLD}ERROR: no user id in response${NC}\n" >&2; exit 1
}
captured "user_id" "$USER_ID"

# =============================================================================
edge_case "Duplicate signup — same email  →  expect 409"
# Re-request OTP with the same email after user is already created
do_request POST "$BASE_URL/users/signup" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${USERNAME}\",\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}"
check_status "409" "Duplicate email → 409"

# =============================================================================
edge_case "Protected route — no auth cookie  →  expect 401"
SAVED_JAR="$COOKIE_JAR"
COOKIE_JAR=""
do_request GET "$BASE_URL/users/$USER_ID"
check_status "401" "No auth cookie → 401"
COOKIE_JAR="$SAVED_JAR"

# =============================================================================
edge_case "Login — wrong password  →  expect 401"
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${EMAIL}\",\"password\":\"wrongpassword\"}"
check_status "401" "Wrong password → 401"

# =============================================================================
edge_case "Login — non-existent email  →  expect 401"
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d '{"email":"nobody@nowhere.com","password":"secret123"}'
check_status "401" "Unknown email → 401"

# =============================================================================
section "4 · Login  →  POST /users/login"
COOKIE_JAR=""
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${EMAIL}\",\"password\":\"${PASSWORD}\"}"
check_status "200" "Login → 200"

# =============================================================================
section "5 · Get user  →  GET /users/$USER_ID"
do_request GET "$BASE_URL/users/$USER_ID"
check_status "200" "Get user → 200"

# =============================================================================
section "6 · Update user  →  PUT /users/$USER_ID"
do_request PUT "$BASE_URL/users/$USER_ID" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"${UPDATED_USERNAME}\",\"email\":\"${UPDATED_EMAIL}\",\"password\":\"${UPDATED_PASSWORD}\"}"
check_status "200" "Update user → 200"

# =============================================================================
section "7 · Shorten URL  →  POST /"
do_request POST "$BASE_URL/" \
  -H "Content-Type: application/json" \
  -d '{"original_url":"https://example.com"}'

SHORT_CODE="$(extract short_code "$RESPONSE_BODY")"
[[ -z "$SHORT_CODE" ]] && {
  printf "${RED}${BOLD}ERROR: no short_code${NC}\n" >&2; exit 1
}
captured "short_code" "$SHORT_CODE"

# =============================================================================
section "8 · Get all URLs  →  GET /"
do_request GET "$BASE_URL/"

# =============================================================================
section "9 · Resolve short URL  →  GET /$SHORT_CODE  (public, expect redirect)"
do_request GET "$BASE_URL/$SHORT_CODE"
[[ "$RESPONSE_STATUS" =~ ^3 ]] && {
  printf "    ${GREEN}${BOLD}✓ PASS${NC}  Short URL redirects → %s\n" "$RESPONSE_STATUS"
  PASS=$((PASS + 1))
} || {
  printf "    ${RED}${BOLD}✗ FAIL${NC}  Expected 3xx redirect, got %s\n" "$RESPONSE_STATUS"
  FAIL=$((FAIL + 1))
}

# =============================================================================
section "10 · Logout  →  POST /users/logout"
do_request POST "$BASE_URL/users/logout"
check_status "204" "Logout → 204"
printf "\n    ${MAGENTA}Cookie jar after logout: '%s'${NC}\n" "$COOKIE_JAR"

# =============================================================================
edge_case "Access protected route after logout  →  expect 401"
do_request GET "$BASE_URL/"
check_status "401" "After logout → 401"

# =============================================================================
section "11 · Re-login for cleanup"
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"${UPDATED_EMAIL}\",\"password\":\"${UPDATED_PASSWORD}\"}"
check_status "200" "Re-login → 200"

# =============================================================================
section "12 · Delete user  →  DELETE /users/$USER_ID"
do_request DELETE "$BASE_URL/users/$USER_ID"
check_status "204" "Delete user → 204"

# =============================================================================
edge_case "Get deleted user  →  expect 404"
do_request GET "$BASE_URL/users/$USER_ID"
check_status "404" "Deleted user → 404"

# =============================================================================
# Summary
TOTAL=$((PASS + FAIL))
printf "\n${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n"
printf "${BOLD}  Results: ${GREEN}${PASS} passed${NC}${BOLD}, ${RED}${FAIL} failed${NC}${BOLD} / ${TOTAL} total${NC}\n"
printf "${BOLD}${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}\n\n"

[[ "$FAIL" -gt 0 ]] && exit 1 || exit 0
