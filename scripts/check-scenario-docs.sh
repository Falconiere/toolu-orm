#!/usr/bin/env bash
# Keeps docs/scenarios/*.md in sync with the test suites.
#
# Every scenario doc has a `## Tests` table with rows `| <lane> | <binary> | <test> |`.
# This script fails when:
#   1. a documented test does not exist in that lane's `cargo nextest list`, or
#   2. a test in a scenario binary (any binary named in a doc) is not documented.
# It only lists tests (compiles, never runs them), so no database is needed.
#
# Lanes (must match CLAUDE.md / .github/workflows/ci.yml):
#   default       cargo nextest list --workspace
#   postgres      cargo nextest list $PKGS --features postgres   (see PKGS below)
#   libsql-only   cargo nextest list -p toolu-orm-query --features libsql
#   rusqlite-only cargo nextest list -p toolu-orm-query --features rusqlite,sqlite-vec
#                 cargo nextest list -p toolu-orm-connection --features rusqlite,sqlite-vec
set -euo pipefail

cd "$(dirname "$0")/.."
DOCS=docs/scenarios
PKGS="-p toolu-orm -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli -p toolu-orm-facade-consumer"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# nextest list prints "    <crate>::<binary> <module::>test"; keep "<binary> <test>".
# A lane that fails to list (compile error) is fatal: an empty list would make
# every documented test look missing and hide the real cause.
list_lane() {
  local lane="$1"; shift
  local raw
  # --color never: CI exports CARGO_TERM_COLOR=always, and ANSI codes would
  # make every line miss the pattern below.
  if ! raw=$(cargo nextest list --color never "$@" 2>&1); then
    echo "scenario-docs: lane '$lane' failed to list tests:" >&2
    printf '%s\n' "$raw" | tail -20 >&2
    return 2
  fi
  printf '%s\n' "$raw" | awk '/^ *[a-z0-9-]+::[a-z0-9_]+ / { sub(/^ *[a-z0-9-]+::/, ""); print }' >> "$tmp/$lane"
}
: > "$tmp/default" "$tmp/postgres" "$tmp/libsql-only" "$tmp/rusqlite-only"
list_lane default --workspace
# shellcheck disable=SC2086
list_lane postgres $PKGS --features postgres
list_lane libsql-only -p toolu-orm-query --features libsql
list_lane rusqlite-only -p toolu-orm-query --features rusqlite,sqlite-vec
list_lane rusqlite-only -p toolu-orm-connection --features rusqlite,sqlite-vec

# Documented rows: "<lane> <binary> <test>" from table rows under each page's
# `## Tests` heading (other tables on the page are ignored).
awk -F'|' '
  FNR == 1 { intests=0 }
  /^## Tests/ { intests=1; next }
  /^## / { intests=0 }
  intests && /^\|/ {
    lane=$2; bin=$3; test=$4
    gsub(/^[ \t]+|[ \t]+$/, "", lane); gsub(/^[ \t]+|[ \t]+$/, "", bin); gsub(/^[ \t]+|[ \t]+$/, "", test)
    gsub(/`/, "", bin); gsub(/`/, "", test)
    if (lane ~ /^(default|postgres|libsql-only|rusqlite-only)$/ && bin != "" && test != "")
      print lane, bin, test
  }' "$DOCS"/*.md | sort -u > "$tmp/documented"

status=0

# 1. Every documented test must exist in its lane.
while read -r lane bin test; do
  if ! grep -qxF "$bin $test" "$tmp/$lane"; then
    echo "scenario-docs: documented test not found in lane '$lane': $bin $test" >&2
    status=1
  fi
done < "$tmp/documented"

# 2. Every test in a documented binary must be documented (any lane).
awk '{print $2}' "$tmp/documented" | sort -u > "$tmp/binaries"
cat "$tmp"/default "$tmp"/postgres "$tmp"/libsql-only "$tmp"/rusqlite-only | sort -u > "$tmp/all"
awk '{print $2, $3}' "$tmp/documented" | sort -u > "$tmp/documented-tests"
while read -r bin; do
  grep -E "^$bin " "$tmp/all" | while read -r b test; do
    if ! grep -qxF "$b $test" "$tmp/documented-tests"; then
      echo "scenario-docs: undocumented test in scenario binary: $b $test" >&2
      echo fail >> "$tmp/undocumented"
    fi
  done
done < "$tmp/binaries"
[ -f "$tmp/undocumented" ] && status=1

if [ "$status" -eq 0 ]; then
  echo "scenario-docs: $(wc -l < "$tmp/documented") documented tests across $(wc -l < "$tmp/binaries") binaries, all in sync"
fi
exit "$status"
