#!/usr/bin/env bash
# Compiles `#[derive(FromRow)]` under every driver combination.
#
# `FromRow` has eight shapes, one per subset of {postgres, libsql, rusqlite},
# and `impl_derived_from_row!` (crates/orm-core/src/row/derived.rs) is defined
# eight times to match. The four CI lanes only ever give orm-core `libsql`,
# `postgres+libsql`, `rusqlite` or `postgres`, so half the definitions are
# never expanded by a lane: a typo in one of the others would ship silently.
#
# orm-macros is the right vehicle. Its features forward straight to orm-core,
# and its `from_row_test` derives `FromRow` with no `required-features`, so
# selecting a combination here expands the derive against the matching
# definition. `--all-targets` is what pulls that test in; without it this
# would only build the proc-macro crate and prove nothing.
#
# `cargo check`, not clippy: the lanes already lint these packages, and this
# script is about whether each combination *resolves* — the inactive drivers'
# decoder blocks are passed to the macro and dropped unexpanded, which is the
# property worth guarding.
set -euo pipefail

cd "$(dirname "$0")/.."

# The empty entry is the no-driver build, where the trait has no methods.
COMBOS=(
  ""
  "postgres"
  "libsql"
  "rusqlite"
  "postgres,libsql"
  "postgres,rusqlite"
  "libsql,rusqlite"
  "postgres,libsql,rusqlite"
)

failed=()
for combo in "${COMBOS[@]}"; do
  label="${combo:-(no driver)}"
  # --no-default-features so orm-core gets exactly this set; its default is
  # libsql, which would otherwise contaminate every combination.
  args=(check -q -p toolu-orm-macros --no-default-features --all-targets)
  [[ -n $combo ]] && args+=(--features "$combo")

  printf '%-26s ' "$label"
  if log=$(cargo "${args[@]}" 2>&1); then
    echo "ok"
  else
    echo "FAILED"
    printf '%s\n' "$log" | sed 's/^/    /'
    failed+=("$label")
  fi
done

if ((${#failed[@]})); then
  echo "derive-matrix: ${#failed[@]} combination(s) failed: ${failed[*]}" >&2
  exit 1
fi

echo "derive-matrix: all ${#COMBOS[@]} driver combinations expand the FromRow derive"
