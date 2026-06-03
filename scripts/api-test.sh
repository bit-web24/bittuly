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
    -d "{\"username\":\"bittu-$STAMP\",\"email\":\"bittu-$STAMP@example.com\",\"password\":\"secret\"}"
)"
echo "$CREATE_USER_RESPONSE"

USER_ID="$(printf "%s" "$CREATE_USER_RESPONSE" | extract_json_string id)"
if [ -z "$USER_ID" ]; then
  echo "failed to extract user id from create user response" >&2
  exit 1
fi

echo
echo "2. Get user by id: $USER_ID"
curl -sS -X GET "$BASE_URL/users/$USER_ID"
echo

echo
echo "3. Update user by id: $USER_ID"
UPDATE_USER_RESPONSE="$(
  curl -sS -X PUT "$BASE_URL/users/$USER_ID" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"bittu-updated-$STAMP\",\"email\":\"bittu-updated-$STAMP@example.com\",\"password\":\"secret-updated\"}"
)"
echo "$UPDATE_USER_RESPONSE"

echo
echo "4. Shorten URL for user id: $USER_ID"
SHORTEN_URL_RESPONSE="$(
  curl -sS -X POST "$BASE_URL/" \
    -H "Content-Type: application/json" \
    -d "{\"original_url\":\"https://example.com\",\"user_id\":\"$USER_ID\"}"
)"
echo "$SHORTEN_URL_RESPONSE"

SHORT_CODE="$(printf "%s" "$SHORTEN_URL_RESPONSE" | extract_json_string short_code)"
if [ -z "$SHORT_CODE" ]; then
  echo "failed to extract short_code from shorten url response" >&2
  exit 1
fi

echo
echo "5. Get all URLs for user id: $USER_ID"
curl -sS -X GET "$BASE_URL/" \
  -H "Content-Type: application/json" \
  -d "{\"user_id\":\"$USER_ID\"}"
echo

echo
echo "6. Resolve short URL: $SHORT_CODE"
curl -sS -i -X GET "$BASE_URL/$SHORT_CODE"
echo

echo
echo "7. Delete user by id: $USER_ID"
curl -sS -i -X DELETE "$BASE_URL/users/$USER_ID"
echo
