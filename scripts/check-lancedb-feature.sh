#!/usr/bin/env bash
# Prove the public Lance feature resolves the pinned binding and cannot select
# a legacy query executor when combined with another backend.
set -euo pipefail

cd "$(dirname "$0")/.."
repo_dir=$PWD

cargo check --locked -p toolu-orm-facade-consumer --features lancedb --all-targets
tree=$(cargo tree --locked -p toolu-orm-facade-consumer --features lancedb \
  -e features -i duckdb)
if [[ "$tree" != *'duckdb v1.10505.0'* ]] ||
  [[ "$tree" != *'duckdb feature "bundled"'* ]] ||
  [[ "$tree" != *'toolu-orm-facade-consumer'* ]]; then
  printf 'lancedb-feature: facade does not resolve pinned DuckDB\n' >&2
  exit 1
fi

for driver in postgres rusqlite libsql; do
  cargo check --locked -p toolu-orm --features "lancedb,$driver"
done

fixture_dir=$(mktemp -d)
trap 'rm -rf "$fixture_dir"' EXIT
mkdir -p "$fixture_dir/src"
# Keep the temporary consumer on the same resolved dependency versions as the
# workspace; otherwise its fresh lockfile selects another native DuckDB build.
cp "$repo_dir/Cargo.lock" "$fixture_dir/Cargo.lock"

assert_unavailable() {
  local driver=$1
  local item=$2
  cat > "$fixture_dir/Cargo.toml" <<EOF
[package]
name = "lancedb-feature-negative"
version = "0.0.0"
edition = "2021"

[dependencies]
toolu-orm = { path = "$repo_dir/crates/orm", default-features = false, features = ["lancedb", "$driver"] }
EOF
  if [[ "$item" == Driver ]]; then
    cat > "$fixture_dir/src/lib.rs" <<'EOF'
use toolu_orm::query::QueryError;

pub fn accepts_error(error: QueryError) {
  match error {
    QueryError::Driver(_) => {}
    _ => {}
  }
}
EOF
  else
    printf 'use toolu_orm::query::%s;\n' "$item" > "$fixture_dir/src/lib.rs"
  fi

  if CARGO_TARGET_DIR="$repo_dir/target" cargo check --offline \
    --manifest-path "$fixture_dir/Cargo.toml" > "$fixture_dir/build.log" 2>&1; then
    printf 'lancedb-feature: %s unexpectedly exposes query::%s\n' "$driver" "$item" >&2
    exit 1
  fi

  case "$item" in
    executor|transaction)
      expected=$(printf 'unresolved import `%s`' "toolu_orm::query::$item")
      ;;
    Driver) expected='no variant or associated item named `Driver` found' ;;
    *) printf 'lancedb-feature: unknown import %s\n' "$item" >&2; exit 1 ;;
  esac
  if ! rg --fixed-strings --quiet "$expected" "$fixture_dir/build.log"; then
    tail -30 "$fixture_dir/build.log" >&2
    printf 'lancedb-feature: %s failed for a reason other than absent query::%s\n' \
      "$driver" "$item" >&2
    exit 1
  fi
  printf 'lancedb-feature: lancedb,%s has no query::%s\n' "$driver" "$item"
}

assert_unavailable postgres executor
assert_unavailable rusqlite executor
assert_unavailable libsql executor
assert_unavailable libsql transaction
assert_unavailable postgres Driver
assert_unavailable libsql Driver
