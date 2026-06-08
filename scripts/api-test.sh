#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:3000}"
STAMP="$(date +%s)"

# ── colours ───────────────────────────────────────────────────────────────────
BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; MAGENTA='\033[0;35m'

# ── state ─────────────────────────────────────────────────────────────────────
RESPONSE_BODY=""
RESPONSE_HEADERS=""
# Cookie jar: "access_token=eyJ...; refresh_token=eyJ..."
COOKIE_JAR=""

# ── helpers ───────────────────────────────────────────────────────────────────
section() {
  local width=62
  local line; line="$(printf '─%.0s' $(seq 1 $width))"
  printf "\n${BOLD}${BLUE}┌%s┐${NC}\n" "$line"
  printf   "${BOLD}${BLUE}│  %-60s│${NC}\n" "$*"
  printf   "${BOLD}${BLUE}└%s┘${NC}\n" "$line"
}

kv() {   # kv label value [colour]
  local c="${3:-$NC}"
  printf "    ${DIM}%-18s${NC}${c}%s${NC}\n" "$1:" "$2"
}

pretty_json() {
  if command -v jq &>/dev/null; then
    printf '%s' "$1" | jq . 2>/dev/null && return
  fi
  printf '%s\n' "$1"
}

# Parse a cookie value from the cookie jar string.
# usage: cookie_get "access_token"
cookie_get() {
  printf '%s' "$COOKIE_JAR" \
    | tr ';' '\n' \
    | grep -oP "^\s*$1=\K.*" \
    | head -1 \
    | tr -d ' ' \
    || true
}

# Ingest Set-Cookie headers from RESPONSE_HEADERS into COOKIE_JAR.
update_cookie_jar() {
  while IFS= read -r line; do
    # Match: Set-Cookie: name=value; ...
    if [[ "$line" =~ ^[Ss]et-[Cc]ookie:[[:space:]]*([A-Za-z_]+)=([^;]*) ]]; then
      local name="${BASH_REMATCH[1]}"
      local value="${BASH_REMATCH[2]}"
      if printf '%s' "$line" | grep -qi "Max-Age=0"; then
        # Remove cookie from jar
        COOKIE_JAR="$(printf '%s' "$COOKIE_JAR" \
          | tr ';' '\n' \
          | grep -v "^\s*${name}=" \
          | tr '\n' ';' \
          | sed 's/^;//;s/;$//')"
      else
        # Upsert cookie in jar
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

# do_request METHOD URL [-H "Header"] [-d '{"body"}']
# Prints full request + response. Sets RESPONSE_BODY, RESPONSE_HEADERS.
# Automatically attaches current COOKIE_JAR as Cookie header.
do_request() {
  local method="$1" url="$2"
  shift 2

  local -a curl_args=()
  local -a req_headers=()
  local req_body=""

  # Attach cookie jar if non-empty
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

  # ── print request ──────────────────────────────────────────────────────────
  printf "\n${CYAN}${BOLD}  ▶  REQUEST${NC}\n"
  kv "Method"  "$method" "${BOLD}"
  kv "URL"     "$url"    "${BOLD}"
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

  # ── execute ────────────────────────────────────────────────────────────────
  local raw
  raw="$(curl -sS -i -X "$method" "$url" "${curl_args[@]}")"

  # Split: status line / response headers / body
  local status_line
  status_line="$(printf '%s' "$raw" | head -1 | tr -d '\r')"
  RESPONSE_HEADERS="$(printf '%s' "$raw" \
    | awk 'NR==1{next} /^\r?$/{exit} {sub(/\r/,""); print}')"
  RESPONSE_BODY="$(printf '%s' "$raw" \
    | awk 'BEGIN{p=0} /^\r?$/{if(!p){p=1;next}} p{print}')"

  # Update cookie jar from any Set-Cookie headers
  update_cookie_jar

  local status_code
  status_code="$(printf '%s' "$status_line" | grep -oE '[0-9]{3}' | head -1)"
  local sc_color="$GREEN"
  [[ "$status_code" =~ ^3 ]] && sc_color="$YELLOW"
  [[ "$status_code" =~ ^4 ]] && sc_color="$RED"
  [[ "$status_code" =~ ^5 ]] && sc_color="$RED"

  # ── print response ─────────────────────────────────────────────────────────
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

extract() {  # extract key json → value
  printf '%s' "$2" | grep -oP "\"$1\":\s*\"\K[^\"]+" 2>/dev/null || true
}

captured() {
  printf "\n    ${MAGENTA}${BOLD}Captured:${NC}"
  while [[ $# -ge 2 ]]; do
    printf "${MAGENTA}  %s=%s${NC}" "$1" "$2"; shift 2
  done
  printf "\n"
}

# ═════════════════════════════════════════════════════════════════════════════
section "1 · Create user"
do_request POST "$BASE_URL/users" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"bittu-$STAMP\",\"email\":\"bittu-$STAMP@example.com\",\"password\":\"secret123\"}"

USER_ID="$(extract id "$RESPONSE_BODY")"
[[ -z "$USER_ID" ]] && { printf "${RED}ERROR: no user id${NC}\n" >&2; exit 1; }
captured "id" "$USER_ID"

# ═════════════════════════════════════════════════════════════════════════════
section "2 · Login  (cookie jar refreshed)"
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"bittu-$STAMP@example.com\",\"password\":\"secret123\"}"

# ═════════════════════════════════════════════════════════════════════════════
section "3 · Get user  →  /users/$USER_ID"
do_request GET "$BASE_URL/users/$USER_ID"

# ═════════════════════════════════════════════════════════════════════════════
section "4 · Update user  →  /users/$USER_ID"
do_request PUT "$BASE_URL/users/$USER_ID" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"bittu-updated-$STAMP\",\"email\":\"bittu-updated-$STAMP@example.com\",\"password\":\"secret-updated\"}"

# ═════════════════════════════════════════════════════════════════════════════
section "5 · Shorten URL  →  POST /"
do_request POST "$BASE_URL/" \
  -H "Content-Type: application/json" \
  -d '{"original_url":"https://example.com"}'

SHORT_CODE="$(extract short_code "$RESPONSE_BODY")"
[[ -z "$SHORT_CODE" ]] && { printf "${RED}ERROR: no short_code${NC}\n" >&2; exit 1; }
captured "short_code" "$SHORT_CODE"

# ═════════════════════════════════════════════════════════════════════════════
section "6 · Get all URLs  →  GET /"
do_request GET "$BASE_URL/"

# ═════════════════════════════════════════════════════════════════════════════
section "7 · Resolve short URL  →  /$SHORT_CODE  (public)"
do_request GET "$BASE_URL/$SHORT_CODE"

# ═════════════════════════════════════════════════════════════════════════════
section "8 · Logout  →  POST /users/logout"
do_request POST "$BASE_URL/users/logout"
printf "\n    ${MAGENTA}Cookie jar after logout: '%s'${NC}\n" "$COOKIE_JAR"

# ═════════════════════════════════════════════════════════════════════════════
section "9 · Verify logout  →  GET / (expect 401)"
do_request GET "$BASE_URL/"

# ═════════════════════════════════════════════════════════════════════════════
# Re-login to get fresh cookies for cleanup
section "10 · Re-login for cleanup"
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"bittu-updated-$STAMP@example.com\",\"password\":\"secret-updated\"}"

# ═════════════════════════════════════════════════════════════════════════════
section "11 · Delete user  →  /users/$USER_ID"
do_request DELETE "$BASE_URL/users/$USER_ID"

printf "\n${BOLD}${GREEN}✓ All steps completed.${NC}\n\n"
