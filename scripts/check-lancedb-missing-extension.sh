#!/usr/bin/env bash
# Prove the CI startup command fails when its provisioned Lance extension is absent.
set -euo pipefail

cd "$(dirname "$0")/.."

fixture_dir=$(mktemp -d)
trap 'rm -rf "$fixture_dir"' EXIT
missing="$fixture_dir/missing.duckdb_extension"
output="$fixture_dir/nextest.log"

if LANCE_EXTENSION_PATH="$missing" cargo nextest run \
  -p toolu-orm-connection --no-default-features --features lancedb \
  -E 'test(=pinned_extension_prepares_lance_sql_after_startup)' \
  --color never --failure-output immediate \
  >"$output" 2>&1; then
  cat "$output" >&2
  printf 'lancedb-missing-extension: startup suite passed without an extension\n' >&2
  exit 1
fi

if ! grep -Fq 'Error: LanceDependencyUnavailable(' "$output" || \
  ! grep -Eq 'FAIL.*pinned_extension_prepares_lance_sql_after_startup' "$output"; then
  cat "$output" >&2
  printf 'lancedb-missing-extension: failure was not the expected startup error\n' >&2
  exit 1
fi

printf 'lancedb-missing-extension: startup fails with LanceDependencyUnavailable\n'
