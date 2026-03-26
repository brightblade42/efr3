#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
PG_BIN="${PG_BIN:-/opt/homebrew/opt/libpq/bin}"
DB_URL="${IDENTITY_DB_URL:-}"

if [[ -z "$DB_URL" ]]; then
  echo "IDENTITY_DB_URL is required"
  echo "Example: export IDENTITY_DB_URL='postgresql://<user>:<password>@<host>:5432/identity?sslmode=prefer'"
  exit 1
fi

if [[ ! -x "$PG_BIN/pg_dump" || ! -x "$PG_BIN/psql" ]]; then
  echo "Missing pg_dump/psql in PG_BIN=$PG_BIN"
  echo "Install libpq 18+ and/or set PG_BIN"
  exit 1
fi

FULL_OUT="$ROOT_DIR/identity_schema_full.sql"
OWNED_OUT="$ROOT_DIR/identity_schema_owned.sql"

"$PG_BIN/pg_dump" --schema-only --no-owner --no-privileges "$DB_URL" -n eyefr -n logs -n public -f "$FULL_OUT"
"$PG_BIN/pg_dump" --schema-only --no-owner --no-privileges "$DB_URL" -n eyefr -n logs -f "$OWNED_OUT"

strip_dump_tokens() {
  local file="$1"
  local tmp="${file}.tmp"
  awk '!/^\\restrict / && !/^\\unrestrict / { print }' "$file" > "$tmp"
  mv "$tmp" "$file"
}

strip_dump_tokens "$FULL_OUT"
strip_dump_tokens "$OWNED_OUT"

SERVER_VERSION="$($PG_BIN/psql "$DB_URL" -tAc "show server_version;" | tr -d '[:space:]')"
CAPTURED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

cat > "$ROOT_DIR/SCHEMA_SOURCE.md" <<EOF
# Schema Source

- Captured at: $CAPTURED_AT
- Database: identity
- Server version: $SERVER_VERSION
- Snapshots:
  - identity_schema_owned.sql (eyefr, logs)
  - identity_schema_full.sql (eyefr, logs, public)
EOF

echo "Refreshed schema snapshots in $ROOT_DIR"
