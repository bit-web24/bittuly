#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:3000}"
STAMP="$(date +%s)"

# ── colours ───────────────────────────────────────────────────────────────────
BOLD='\033[1m'; DIM='\033[2m'; NC='\033[0m'
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[0;33m'
BLUE='\033[0;34m'; CYAN='\033[0;36m'; MAGENTA='\033[0;35m'

# ── helpers ───────────────────────────────────────────────────────────────────
RESPONSE_BODY=""

section() {
  local title="$*"
  local width=60
  local line
  line="$(printf '─%.0s' $(seq 1 $width))"
  printf "\n${BOLD}${BLUE}┌%s┐${NC}\n" "$line"
  printf   "${BOLD}${BLUE}│  %-58s│${NC}\n" "$title"
  printf   "${BOLD}${BLUE}└%s┘${NC}\n" "$line"
}

kv() {  # kv label value [color]
  local color="${3:-$NC}"
  printf "    ${DIM}%-16s${NC}${color}%s${NC}\n" "$1:" "$2"
}

pretty_json() {
  if command -v jq &>/dev/null; then
    printf '%s' "$1" | jq . 2>/dev/null && return
  fi
  printf '%s\n' "$1"
}

# do_request METHOD URL [-H "Header"] [-d '{"body"}']
# Prints full request + response. Stores response body in RESPONSE_BODY.
do_request() {
  local method="$1" url="$2"
  shift 2

  local -a curl_args=()
  local -a req_headers=()
  local req_body=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -H) req_headers+=("$2"); curl_args+=(-H "$2"); shift 2 ;;
      -d) req_body="$2";       curl_args+=(-d "$2"); shift 2 ;;
      *)  curl_args+=("$1");   shift ;;
    esac
  done

  # ── request ──────────────────────────────────────────────────────────────
  printf "\n${CYAN}${BOLD}  ▶  REQUEST${NC}\n"
  kv "Method" "$method"  "${BOLD}"
  kv "URL"    "$url"     "${BOLD}"

  if [[ ${#req_headers[@]} -gt 0 ]]; then
    printf "    ${DIM}%-16s${NC}\n" "Headers"
    for h in "${req_headers[@]}"; do
      printf "      ${YELLOW}%s${NC}\n" "$h"
    done
  fi

  if [[ -n "$req_body" ]]; then
    printf "    ${DIM}%-16s${NC}\n" "Body"
    pretty_json "$req_body" | sed 's/^/      /'
  fi

  # ── execute ───────────────────────────────────────────────────────────────
  local raw
  raw="$(curl -sS -i -X "$method" "$url" "${curl_args[@]}")"

  # split: status line / response headers / response body
  local status_line resp_hdrs resp_body
  status_line="$(printf '%s' "$raw" | head -1 | tr -d '\r')"
  resp_hdrs="$(printf '%s' "$raw" \
    | awk 'NR==1{next} /^\r?$/{exit} {sub(/\r/,""); print}')"
  resp_body="$(printf '%s' "$raw" \
    | awk 'BEGIN{p=0} /^\r?$/{if(!p){p=1;next}} p{print}')"

  local status_code
  status_code="$(printf '%s' "$status_line" | grep -oE '[0-9]{3}' | head -1)"

  local sc_color="$GREEN"
  [[ "$status_code" =~ ^3 ]] && sc_color="$YELLOW"
  [[ "$status_code" =~ ^4 ]] && sc_color="${RED}"
  [[ "$status_code" =~ ^5 ]] && sc_color="${RED}"

  # ── response ──────────────────────────────────────────────────────────────
  printf "\n${GREEN}${BOLD}  ◀  RESPONSE${NC}\n"
  kv "Status" "$status_line" "${BOLD}${sc_color}"

  if [[ -n "$resp_hdrs" ]]; then
    printf "    ${DIM}%-16s${NC}\n" "Headers"
    while IFS= read -r line; do
      printf "      ${YELLOW}%s${NC}\n" "$line"
    done <<< "$resp_hdrs"
  fi

  if [[ -n "$resp_body" ]]; then
    printf "    ${DIM}%-16s${NC}\n" "Body"
    if command -v jq &>/dev/null \
        && printf '%s' "$resp_body" | jq . &>/dev/null 2>&1; then
      printf '%s' "$resp_body" | jq . | sed 's/^/      /'
    else
      printf "      %s\n" "$resp_body"
    fi
  fi

  RESPONSE_BODY="$resp_body"
}

extract() {  # extract key json  →  prints value
  local key="$1" json="$2"
  printf '%s' "$json" | grep -oP "\"$key\":\s*\"\K[^\"]+" 2>/dev/null || true
}

captured() {  # pretty-print what we captured from the response
  printf "\n    ${MAGENTA}${BOLD}Captured:${NC}"
  while [[ $# -ge 2 ]]; do
    printf "${MAGENTA}  %s=%s${NC}" "$1" "$2"
    shift 2
  done
  printf "\n"
}

# ═════════════════════════════════════════════════════════════════════════════
section "1 · Create user"
do_request POST "$BASE_URL/users" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"bittu-$STAMP\",\"email\":\"bittu-$STAMP@example.com\",\"password\":\"secret123\"}"

USER_ID="$(extract id            "$RESPONSE_BODY")"
AUTH_TOKEN="$(extract token      "$RESPONSE_BODY")"
REFRESH_TOKEN="$(extract refresh_token "$RESPONSE_BODY")"

[[ -z "$USER_ID" ]]       && { printf "${RED}ERROR: no user id${NC}\n"      >&2; exit 1; }
[[ -z "$AUTH_TOKEN" ]]    && { printf "${RED}ERROR: no token${NC}\n"        >&2; exit 1; }
[[ -z "$REFRESH_TOKEN" ]] && { printf "${RED}ERROR: no refresh_token${NC}\n" >&2; exit 1; }

captured "id" "$USER_ID"

# ═════════════════════════════════════════════════════════════════════════════
section "2 · Login"
do_request POST "$BASE_URL/users/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"bittu-$STAMP@example.com\",\"password\":\"secret123\"}"

# ═════════════════════════════════════════════════════════════════════════════
section "3 · Get user  →  /users/$USER_ID"
do_request GET "$BASE_URL/users/$USER_ID" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "x-refresh-token: $REFRESH_TOKEN"

# ═════════════════════════════════════════════════════════════════════════════
section "4 · Update user  →  /users/$USER_ID"
do_request PUT "$BASE_URL/users/$USER_ID" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "x-refresh-token: $REFRESH_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"bittu-updated-$STAMP\",\"email\":\"bittu-updated-$STAMP@example.com\",\"password\":\"secret-updated\"}"

UPDATED_TOKEN="$(extract token         "$RESPONSE_BODY" || true)"
UPDATED_REFRESH="$(extract refresh_token "$RESPONSE_BODY" || true)"
[[ -n "$UPDATED_TOKEN" ]]   && AUTH_TOKEN="$UPDATED_TOKEN"
[[ -n "$UPDATED_REFRESH" ]] && REFRESH_TOKEN="$UPDATED_REFRESH"

# ═════════════════════════════════════════════════════════════════════════════
section "5 · Shorten URL  →  POST /"
do_request POST "$BASE_URL/" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "x-refresh-token: $REFRESH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"original_url":"https://example.com"}'

SHORT_CODE="$(extract short_code "$RESPONSE_BODY")"
[[ -z "$SHORT_CODE" ]] && { printf "${RED}ERROR: no short_code${NC}\n" >&2; exit 1; }
captured "short_code" "$SHORT_CODE"

# ═════════════════════════════════════════════════════════════════════════════
section "6 · Get all URLs  →  GET /"
do_request GET "$BASE_URL/" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "x-refresh-token: $REFRESH_TOKEN"

# ═════════════════════════════════════════════════════════════════════════════
section "7 · Resolve short URL  →  /$SHORT_CODE  (public)"
do_request GET "$BASE_URL/$SHORT_CODE"

# ═════════════════════════════════════════════════════════════════════════════
section "8 · Delete user  →  /users/$USER_ID"
do_request DELETE "$BASE_URL/users/$USER_ID" \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "x-refresh-token: $REFRESH_TOKEN"

printf "\n${BOLD}${GREEN}✓ All steps completed.${NC}\n\n"
