#!/usr/bin/env bash
# Compiles `#[derive(FromRow)]` under every driver combination.
#
# `FromRow` has eight shapes, one per subset of {postgres, libsql, rusqlite},
# and `impl_derived_from_row!` (crates/orm-core/src/row/derived.rs) is defined
# eight times to match. The four CI lanes only ever give orm-core `libsql`,
# `postgres+libsql`, `rusqlite` or `postgres`, so half the definitions are
# never expanded by a lane: a typo in one of the others would ship silently.
#
# Two packages, because they prove different things:
#
#   toolu-orm-facade-consumer  The decisive case. `toolu-orm` is its only
#                              dependency, so in the rusqlite-only build
#                              `tokio-postgres` is absent from its dependency
#                              graph entirely — yet the derive, which emits a
#                              postgres decoder naming `tokio_postgres::Row`
#                              unconditionally, still compiles. That is the
#                              token-dropping claim in `derived.rs`'s docs
#                              under test rather than merely asserted.
#   toolu-orm-macros           The wider derive surface: `#[from_row(with)]`,
#                              renamed columns, several structs. Its
#                              dev-dependencies pull in all three driver
#                              crates, so it cannot prove absence — it proves
#                              the generated code itself type-checks.
#
# `--all-targets` is what pulls in the tests that do the deriving; without it
# this would build library code that never invokes the macro and prove nothing.
#
# `cargo check`, not clippy: the lanes already lint these packages, and the
# property here is whether each combination *resolves*.
set -euo pipefail

cd "$(dirname "$0")/.."

PACKAGES=(toolu-orm-facade-consumer toolu-orm-macros)

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
for pkg in "${PACKAGES[@]}"; do
  echo "$pkg"
  for combo in "${COMBOS[@]}"; do
    label="${combo:-(no driver)}"
    # --no-default-features so orm-core gets exactly this set; its default is
    # libsql, which would otherwise contaminate every combination.
    args=(check -q -p "$pkg" --no-default-features --all-targets)
    [[ -n $combo ]] && args+=(--features "$combo")

    printf '  %-26s ' "$label"
    if log=$(cargo "${args[@]}" 2>&1); then
      echo "ok"
    else
      echo "FAILED"
      printf '%s\n' "$log" | sed 's/^/      /'
      failed+=("$pkg $label")
    fi
  done
done

if ((${#failed[@]})); then
  printf 'derive-matrix: %d combination(s) failed:\n' "${#failed[@]}" >&2
  printf '  %s\n' "${failed[@]}" >&2
  exit 1
fi

echo "derive-matrix: the FromRow derive expands on all ${#COMBOS[@]} driver" \
  "combinations, in ${#PACKAGES[@]} packages"
