#!/usr/bin/env bash
# Run issue #146's isolated Rust smoke test against a pinned DuckDB core extension.
set -euo pipefail

cd "$(dirname "$0")/.."

dependency_error() {
  printf 'LanceDependencyUnavailable: %s\n' "$1" >&2
  exit 1
}

case "$(uname -s)/$(uname -m)" in
  Darwin/arm64)
    platform=osx_arm64
    platform_version=$(sw_vers -productVersion)
    expected=9c4e51633101be4e51d6a790859a2464d115784eda767608708e23ea5e955b19
    ;;
  Linux/x86_64)
    platform=linux_amd64
    if [[ ! -r /etc/os-release ]]; then
      dependency_error "cannot read Linux OS release metadata"
    fi
    platform_version=$(source /etc/os-release && printf '%s' "${VERSION_ID:-}") || \
      dependency_error "cannot read Linux OS release metadata"
    if [[ -z "$platform_version" ]]; then
      dependency_error "Linux OS release version is missing"
    fi
    expected=cae58f5c0831454b44b973875d0d457bf96574b0c629875b732ef4578fb07080
    ;;
  *)
    dependency_error "lance extension has no pinned artifact for $(uname -s)/$(uname -m)"
    ;;
esac

artifact_dir=$(mktemp -d) || dependency_error "cannot create temporary artifact directory"
trap 'rm -rf "$artifact_dir"' EXIT
url="https://extensions.duckdb.org/v1.5.5/$platform/lance.duckdb_extension.gz"
compressed="$artifact_dir/lance.duckdb_extension.gz"
extension="$artifact_dir/lance.duckdb_extension"

printf 'Lance smoke platform: %s/%s (%s), OS version %s\n' \
  "$(uname -s)" "$(uname -m)" "$platform" "$platform_version"
if ! curl --fail --location --silent --show-error --connect-timeout 10 --max-time 180 \
  "$url" --output "$compressed"; then
  dependency_error "cannot download pinned lance extension from $url"
fi
if ! checksum=$(shasum -a 256 "$compressed"); then
  dependency_error "cannot hash pinned lance extension"
fi
actual=${checksum%% *}
if [[ "$actual" != "$expected" ]]; then
  dependency_error "lance extension checksum mismatch: expected $expected, found $actual"
fi
if ! gzip -dc "$compressed" > "$extension"; then
  dependency_error "cannot decompress pinned lance extension"
fi

printf 'Lance extension SHA-256: %s\n' "$actual"
cargo fmt --manifest-path probes/lancedb/Cargo.toml -- --check
cargo clippy --manifest-path probes/lancedb/Cargo.toml --locked --all-targets -- -D warnings

if ! listed=$(cargo nextest list --manifest-path probes/lancedb/Cargo.toml --locked --color never); then
  printf 'lancedb-smoke: cannot list Rust tests\n' >&2
  exit 1
fi
actual=$(printf '%s\n' "$listed" | awk '
  /^toolu-orm-lancedb-probe::lancedb_(smoke|capability)_test / {
    sub(/^toolu-orm-lancedb-probe::/, "")
    print
  }
' | sort)
documented=$(awk -F'|' '
  $2 ~ /^[[:space:]]*lancedb-smoke[[:space:]]*$/ {
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", $3)
    gsub(/^[[:space:]]+|[[:space:]]+$/, "", $4)
    print $3 " " $4
  }
' docs/scenarios/lancedb-rust-smoke.md docs/scenarios/lancedb-ddl-constraints-search.md | sort)
if [[ -z "$actual" || -z "$documented" ]] || \
  ! diff -u <(printf '%s\n' "$documented") <(printf '%s\n' "$actual"); then
  printf 'lancedb-smoke: scenario docs and Rust test names differ\n' >&2
  exit 1
fi

LANCE_EXTENSION_PATH="$extension" cargo nextest run \
  --manifest-path probes/lancedb/Cargo.toml --locked --success-output immediate
