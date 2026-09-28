#!/usr/bin/env bash
# Compiles `#[derive(FromRow)]` under all sixteen driver combinations.
#
# `FromRow` has eight shapes, one per subset of {postgres, libsql, rusqlite},
# and `impl_derived_from_row!` (crates/orm-core/src/row/derived.rs) is defined
# eight times to match; a core-selected helper adds the Lance decoder.
# The four relational CI lanes only ever give orm-core `libsql`,
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
  "lancedb"
  "lancedb,postgres"
  "lancedb,libsql"
  "lancedb,rusqlite"
  "lancedb,postgres,libsql"
  "lancedb,postgres,rusqlite"
  "lancedb,libsql,rusqlite"
  "lancedb,postgres,libsql,rusqlite"
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

# A consumer with no local features catches cfgs accidentally emitted at the
# expansion site. An i32 field without Lance also proves inactive Lance tokens
# impose no FromLanceValue bound (i32 is not a supported Lance scalar).
consumer=$(mktemp -d)
trap 'rm -rf "$consumer"' EXIT
mkdir "$consumer/src"
cp Cargo.lock "$consumer/Cargo.lock"
for driver in lancedb rusqlite; do
  cat > "$consumer/Cargo.toml" <<TOML
[package]
name = "derive-without-local-features"
version = "0.0.0"
edition = "2021"
[workspace]
[dependencies]
toolu-orm = { path = "$PWD/crates/orm", default-features = false, features = ["$driver"] }
[profile.dev]
split-debuginfo = "unpacked"
debug = 1
codegen-units = 256
[profile.dev.package."*"]
debug = 0
[profile.dev.package.sha2]
opt-level = 2
[profile.dev.package.serde_json]
opt-level = 2
TOML
  scalar=i64
  [[ "$driver" == rusqlite ]] && scalar=i32
  cat > "$consumer/src/lib.rs" <<RUST
#![deny(unexpected_cfgs)]
#[derive(toolu_orm::FromRow)]
pub struct Row {
    pub id: $scalar,
    pub label: Option<String>,
}
RUST
  CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}" \
    cargo check -q --offline --manifest-path "$consumer/Cargo.toml"
  printf 'derive-matrix: external %s consumer without local features ok\n' "$driver"
done
