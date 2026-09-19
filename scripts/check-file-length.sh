#!/usr/bin/env bash
# Fails when a Rust file is longer than the 250-line cap CLAUDE.md sets.
#
# Scope: every *.rs file under crates/ — src/, tests/ (shared fixtures and the
# trybuild inputs included), and build.rs. No path is exempt and there is no
# ignore-comment: an escape hatch would be the rule going back to aspirational,
# one level down.
#
# Why tests/ counts too. CLAUDE.md names src/ explicitly in the rules that mean
# src/ ("No .unwrap() ... in src/; Allowed in tests/"); the cap says "per file".
# The repository already applies it there — tests/select_test/, tests/sql_test/
# and tests/rusqlite_maintenance_test/ are all folder modules because one file
# had outgrown the cap — and when this check was written five of the six files
# over it were test files, the longest at 304 lines. A src/-only check would
# have gone green on that tree, which is the drift it exists to stop.
#
# The remedy differs by directory, so the message names it: a src/ file becomes
# a folder module whose mod.rs holds only mod, pub use and //! docs, while a
# flat test file becomes tests/<name>_test/ whose entry file must be main.rs — a
# mod.rs there is silently never compiled (issue #125) — plus a [[test]] path
# update in the five crates that set autotests = false.
#
# wc -l undercounts a file with no trailing newline by one. `cargo fmt --all --
# --check` runs before this in every lane and in CI, and rustfmt always writes
# that newline, so on any tree that reaches this check the two agree.
#
# Exit codes: 0 every file within the cap; 1 at least one over; 2 the check
# could not run.
set -euo pipefail

cd "$(dirname "$0")/.."

CAP=250

status=0
checked=0
longest=0
longest_file=

# -print0 so a path containing a space or a newline stays one path.
while IFS= read -r -d '' file; do
  lines=$(wc -l < "$file" | tr -d ' ')
  checked=$((checked + 1))
  if [ "$lines" -gt "$longest" ]; then
    longest=$lines
    longest_file=$file
  fi
  [ "$lines" -le "$CAP" ] && continue

  case "$file" in
    */tests/*/*)
      remedy="move part of it into a new sibling module and declare that module in its binary's main.rs"
      ;;
    */tests/*)
      remedy="split it into tests/<name>_test/ with main.rs as the entry file (never mod.rs), moving the [[test]] path with it"
      ;;
    *)
      remedy="split it into a folder module whose mod.rs holds only mod, pub use and //! docs"
      ;;
  esac
  echo "file-length: $file: $lines lines exceeds the $CAP-line cap — $remedy" >&2
  status=1
done < <(find crates -name '*.rs' -not -path '*/target/*' -print0)

# An enumeration that found nothing must never read as "every file is fine".
if [ "$checked" -eq 0 ]; then
  echo "file-length: no Rust file found under crates/ — is this the repository root?" >&2
  exit 2
fi

if [ "$status" -eq 0 ]; then
  echo "file-length: $checked Rust files under crates/, longest $longest ($longest_file), cap $CAP"
fi
exit "$status"
