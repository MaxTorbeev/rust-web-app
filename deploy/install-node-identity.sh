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

# Атомарно записывает identity в файл: временный файл в том же каталоге,
# права 0600, затем переименование.
write_identity_file() {
  local node_id="$1"
  local temporary_file

  temporary_file="$(mktemp "$deployment_directory/.node.env.XXXXXX")"

  cleanup() {
    rm -f "$temporary_file"
  }

  trap cleanup EXIT

  printf 'APP_NODE_ID=%s\n' "$node_id" > "$temporary_file"
  chmod 0600 "$temporary_file"
  mv "$temporary_file" "$identity_file"

  trap - EXIT
}

# Проверяет существующий identity-файл.
#
# Файл никогда не перезаписывается: node id участвует в owner lease и ключах
# внешних систем. Допустима ровно одна строка `APP_NODE_ID=<id>`; любое другое
# содержимое — ошибка, которую исправляют вручную, сохранив значение id.
validate_identity_file() {
  local identity_line
  local line_count

  [[ ! -L "$identity_file" ]] \
    || fail "$identity_file must not be a symbolic link"

  [[ -f "$identity_file" ]] \
    || fail "$identity_file is not a regular file"

  line_count="$(wc -l < "$identity_file" | tr -d '[:space:]')"

  [[ "$line_count" = "1" ]] \
    || fail "$identity_file must contain exactly one line: APP_NODE_ID=<id>"

  IFS= read -r identity_line < "$identity_file"

  [[ "$identity_line" =~ ^APP_NODE_ID=[A-Za-z0-9][A-Za-z0-9_-]*$ ]] \
    || fail "$identity_file contains an invalid APP_NODE_ID: expected APP_NODE_ID=<id>, found '${identity_line%%=*}=...'"

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

write_identity_file "$node_id"

echo "Realtime node identity created"
