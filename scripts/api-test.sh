#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:3000}"
STAMP="$(date +%s)"

extract_json_string() {
  local key="$1"
  sed -n "s/.*\"$key\":\"\\([^\"]*\\)\".*/\\1/p"
}

echo "1. Create user"
CREATE_USER_RESPONSE="$(
  curl -sS -X POST "$BASE_URL/users" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"bittu-$STAMP\",\"email\":\"bittu-$STAMP@example.com\",\"password\":\"secret123\"}"
)"
echo "$CREATE_USER_RESPONSE"

USER_ID="$(printf "%s" "$CREATE_USER_RESPONSE" | extract_json_string id)"
if [ -z "$USER_ID" ]; then
  echo "failed to extract user id from create user response" >&2
  exit 1
fi

AUTH_TOKEN="$(printf "%s" "$CREATE_USER_RESPONSE" | extract_json_string token)"
if [ -z "$AUTH_TOKEN" ]; then
  echo "failed to extract auth token from create user response" >&2
  exit 1
fi

REFRESH_TOKEN="$(printf "%s" "$CREATE_USER_RESPONSE" | extract_json_string refresh_token)"
if [ -z "$REFRESH_TOKEN" ]; then
  echo "failed to extract refresh token from create user response" >&2
  exit 1
fi

AUTH_HEADER="Authorization: Bearer $AUTH_TOKEN"
REFRESH_HEADER="x-refresh-token: $REFRESH_TOKEN"

echo
echo "2. Login"
LOGIN_RESPONSE="$(
  curl -sS -X POST "$BASE_URL/users/login" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"bittu-$STAMP@example.com\",\"password\":\"secret123\"}"
)"
echo "$LOGIN_RESPONSE"

echo
echo "3. Get user by id: $USER_ID"
curl -sS -X GET "$BASE_URL/users/$USER_ID" \
  -H "$AUTH_HEADER" \
  -H "$REFRESH_HEADER"
echo

echo
echo "4. Update user by id: $USER_ID"
UPDATE_USER_RESPONSE="$(
  curl -sS -X PUT "$BASE_URL/users/$USER_ID" \
    -H "$AUTH_HEADER" \
    -H "$REFRESH_HEADER" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"bittu-updated-$STAMP\",\"email\":\"bittu-updated-$STAMP@example.com\",\"password\":\"secret-updated\"}"
)"
echo "$UPDATE_USER_RESPONSE"

UPDATED_AUTH_TOKEN="$(printf "%s" "$UPDATE_USER_RESPONSE" | extract_json_string token)"
if [ -n "$UPDATED_AUTH_TOKEN" ]; then
  AUTH_TOKEN="$UPDATED_AUTH_TOKEN"
  AUTH_HEADER="Authorization: Bearer $AUTH_TOKEN"
fi

UPDATED_REFRESH_TOKEN="$(printf "%s" "$UPDATE_USER_RESPONSE" | extract_json_string refresh_token)"
if [ -n "$UPDATED_REFRESH_TOKEN" ]; then
  REFRESH_TOKEN="$UPDATED_REFRESH_TOKEN"
  REFRESH_HEADER="x-refresh-token: $REFRESH_TOKEN"
fi

echo
echo "5. Shorten URL"
SHORTEN_URL_RESPONSE="$(
  curl -sS -X POST "$BASE_URL/" \
    -H "$AUTH_HEADER" \
    -H "$REFRESH_HEADER" \
    -H "Content-Type: application/json" \
    -d "{\"original_url\":\"https://example.com\"}"
)"
echo "$SHORTEN_URL_RESPONSE"

SHORT_CODE="$(printf "%s" "$SHORTEN_URL_RESPONSE" | extract_json_string short_code)"
if [ -z "$SHORT_CODE" ]; then
  echo "failed to extract short_code from shorten url response" >&2
  exit 1
fi

echo
echo "6. Get all URLs"
curl -sS -X GET "$BASE_URL/" \
  -H "$AUTH_HEADER" \
  -H "$REFRESH_HEADER"
echo

echo
echo "7. Resolve short URL: $SHORT_CODE (no auth required)"
curl -sS -i -X GET "$BASE_URL/$SHORT_CODE"
echo

echo
echo "8. Delete user by id: $USER_ID"
curl -sS -i -X DELETE "$BASE_URL/users/$USER_ID" \
  -H "$AUTH_HEADER" \
  -H "$REFRESH_HEADER"
echo
