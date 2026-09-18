#!/usr/bin/env bash
# Fails when a test file under crates/*/tests/ is not built by any cargo target.
#
# Three shapes go dark silently, and this repository has produced all three:
#   1. tests/<dir>/ whose entry file is mod.rs rather than main.rs, in a crate
#      that does not name it in a [[test]] path. Auto-discovery only looks at
#      tests/*.rs and tests/<dir>/main.rs, so the whole directory is skipped —
#      issue #125, 33 tests inert for as long as they existed.
#   2. a flat tests/<name>_test.rs with no [[test]] entry, in a crate that sets
#      autotests = false — which is every crate here but orm-core and
#      orm-connection.
#   3. a module file added to a live folder binary that no `mod` declares.
#
# None of the three produces a warning: cargo builds fewer targets, nextest
# lists fewer binaries, and nothing says so. `cargo metadata` knows every target
# cargo *would* build — declared and auto-discovered alike, whatever features a
# lane turns on — and compiles nothing, so this check is a set difference
# against it.
#
# Exit codes: 0 all built; 1 something is not; 2 the check could not run.
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v jq >/dev/null 2>&1; then
  echo "test-targets: jq is required (preinstalled on ubuntu-latest; brew install jq)" >&2
  exit 2
fi

# A metadata failure must not read as "no targets", which would report every
# test file in the workspace as dead and bury the real cause.
if ! meta=$(cargo metadata --no-deps --format-version 1 2>&1); then
  echo "test-targets: cargo metadata failed:" >&2
  printf '%s\n' "$meta" | tail -20 >&2
  exit 2
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# Every test target's entry file, made repo-relative. awk rather than sed: the
# repository path is a literal prefix, and a `.` in it must not be read as a
# regex metacharacter.
printf '%s' "$meta" |
  jq -r '.packages[].targets[] | select(.kind | index("test")) | .src_path' |
  awk -v root="$PWD/" 'index($0, root) == 1 { print substr($0, length(root) + 1); next } { print }' |
  sort -u > "$tmp/targets"

targets=$(wc -l < "$tmp/targets" | tr -d ' ')
if [ "$targets" -eq 0 ]; then
  echo "test-targets: cargo metadata reported no test targets at all" >&2
  exit 2
fi

# True when some test target's entry file lives under $1/. The match sets a
# flag rather than exiting 0 directly: awk runs END even after `exit`, so an
# exit status set in the body is overwritten by END's.
covers_dir() {
  awk -v prefix="$1/" 'index($0, prefix) == 1 { found = 1; exit } END { exit !found }' "$tmp/targets"
}

# True when a sibling .rs file declares module $2 (as `mod x;` / `pub mod x;`,
# or through `#[path = "…x.rs"]`). $1 is the directory, $3 the file itself,
# which is skipped so a file cannot vouch for itself.
declared_in_dir() {
  local dir="$1" stem="$2" self="$3" sibling
  for sibling in "$dir"/*.rs; do
    [ "$sibling" = "$self" ] && continue
    [ -f "$sibling" ] || continue
    if grep -qE "mod[[:space:]]+${stem}[[:space:]]*;" "$sibling" ||
      grep -qF "${stem}.rs" "$sibling"; then
      return 0
    fi
  done
  return 1
}

status=0
checked=0

# Keying on the test attribute rather than on a file-name pattern is what lets
# tests/fixtures/*.rs (shared setup, no #[test] by repository rule) and
# tests/ui/*.rs (trybuild inputs) pass with no allow-list.
while IFS= read -r file; do
  grep -qE '#\[(tokio::)?test\]' "$file" || continue
  checked=$((checked + 1))

  # 1. The file is itself a target entry.
  if grep -qxF "$file" "$tmp/targets"; then
    continue
  fi

  dir=${file%/*}
  tests_root="${file%%/tests/*}/tests"

  # 2a. Some ancestor directory, up to <crate>/tests, holds a target entry.
  # Ancestor-wide rather than parent-only, so a nested module directory
  # (tests/<bin>/<group>/x.rs) is judged by the binary that owns it.
  covered_dir=""
  ancestor=$dir
  while [ "$ancestor" != "$tests_root" ] && [ "$ancestor" != "." ] && [ "$ancestor" != "/" ]; do
    if covers_dir "$ancestor"; then
      covered_dir=$ancestor
      break
    fi
    ancestor=${ancestor%/*}
  done

  if [ -z "$covered_dir" ]; then
    if [ "$dir" = "$tests_root" ]; then
      echo "test-targets: $file: not a cargo test target (this crate sets autotests = false, so add a [[test]] path entry)" >&2
    else
      echo "test-targets: $file: no cargo test target builds this directory (rename its entry file to main.rs, or add a [[test]] path entry)" >&2
    fi
    status=1
    continue
  fi

  # 2b. And a sibling declares it as a module.
  stem=${file##*/}
  stem=${stem%.rs}
  if ! declared_in_dir "$dir" "$stem" "$file"; then
    echo "test-targets: $file: in a built directory but no 'mod $stem' or #[path] declaration names it" >&2
    status=1
  fi
done < <(find crates -path '*/tests/*' -name '*.rs' -not -path '*/target/*' | sort)

if [ "$status" -eq 0 ]; then
  echo "test-targets: $checked test files across $targets cargo test targets, all built"
fi
exit "$status"
