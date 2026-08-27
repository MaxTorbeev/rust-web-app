#!/usr/bin/env bash
set -Eeuo pipefail

# Каталог deployment можно передать первым аргументом. По умолчанию используется
# текущий каталог, поэтому скрипт одинаково работает из cloud-init, CI и вручную.
deployment_directory="${1:-.}"
identity_file="$deployment_directory/node.env"
uuid_source="/proc/sys/kernel/random/uuid"

fail() {
  echo "node identity installation failed: $*" >&2
  exit 1
}

generate_uuid() {
  if [[ -r "$uuid_source" ]]; then
    tr -d '\r\n' < "$uuid_source"
    return
  fi

  if command -v uuidgen >/dev/null 2>&1; then
    uuidgen | tr '[:upper:]' '[:lower:]' | tr -d '\r\n'
    return
  fi

  fail "neither $uuid_source nor uuidgen is available"
}

validate_identity_file() {
  local identity_line
  local line_count

  [[ ! -L "$identity_file" ]] \
    || fail "$identity_file must not be a symbolic link"

  [[ -f "$identity_file" ]] \
    || fail "$identity_file is not a regular file"

  line_count="$(wc -l < "$identity_file" | tr -d '[:space:]')"

  [[ "$line_count" = "1" ]] \
    || fail "$identity_file must contain exactly one line"

  IFS= read -r identity_line < "$identity_file"

  [[ "$identity_line" =~ ^REALTIME_NODE_ID=[A-Za-z0-9][A-Za-z0-9_-]*$ ]] \
    || fail "$identity_file contains an invalid REALTIME_NODE_ID"

  chmod 0600 "$identity_file"
}

[[ -d "$deployment_directory" ]] \
  || fail "deployment directory $deployment_directory does not exist"

if [[ -e "$identity_file" || -L "$identity_file" ]]; then
  validate_identity_file
  echo "Realtime node identity already exists"
  exit 0
fi

node_id="realtime-$(generate_uuid)"

[[ "$node_id" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*$ ]] \
  || fail "generated node identity is invalid"

temporary_file="$(mktemp "$deployment_directory/.node.env.XXXXXX")"

cleanup() {
  rm -f "$temporary_file"
}

trap cleanup EXIT

printf 'REALTIME_NODE_ID=%s\n' "$node_id" > "$temporary_file"
chmod 0600 "$temporary_file"
mv "$temporary_file" "$identity_file"

trap - EXIT

echo "Realtime node identity created"
