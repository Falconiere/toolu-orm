#!/usr/bin/env bash
# Compiles the driver-dependent crates under every driver combination.
#
# `scripts/check-derive-matrix.sh` proves `#[derive(FromRow)]` expands on all
# eight driver combinations. It does not compile orm-connection or orm-query,
# and that gap is what let issue #124 ship: those crates picked a `FromRow`
# method from their *own* feature flags, while the method that exists is decided
# by whatever Cargo unified onto `toolu-orm-core`. The two diverge as soon as one
# crate forwards a driver feature to orm-core but not to its sibling — which
# `crates/orm-query/Cargo.toml` did for `libsql`.
#
# Phase 1 — own features. Each driver-dependent package against all eight
# subsets of {postgres, libsql, rusqlite}. This is the phase that catches #124:
# at `toolu-orm-query --features libsql,rusqlite`, orm-connection receives only
# `rusqlite` while orm-core receives both.
#
# Phase 2 — orm-core superset. orm-connection on one driver while orm-core
# additionally carries another, for all six ordered pairs. No phase-1 cell
# produces this for `libsql_impl.rs`, so without phase 2 that decoder would go
# unchecked against a multi-driver orm-core. It covers orm-connection only:
# orm-query's `select/executor_fetch/` still hand-writes `FromRow` in the
# single-driver shape, which a consumer reaches only by giving orm-core a driver
# it withholds from orm-query — something `CLAUDE.md` forbids and no manifest in
# this repo produces.
#
# `toolu-orm-cli` skips the no-driver cell. Its migration runner decodes rows
# unconditionally (`migrate/store/lookup.rs`, `migrate/pragma_guard.rs`) and the
# zero-driver `FromRow` partition has no methods at all, so `PragmaInt` and
# `AppliedMigration` cannot implement it. orm-cli genuinely requires a driver —
# its `default = ["libsql"]` says so — and that predates this script.
#
# No `--all-targets`, unlike check-derive-matrix.sh where the tests are what
# invoke the macro under test. Here the subject is library code. `--all-targets`
# on a two-driver combination would compile orm-query test binaries whose
# `required-features` are satisfied but whose bodies call `T::from_row` and
# `SelectBuilder::fetch_all` — exactly what `cfg_single_backend!` removes from a
# two-driver build.
#
# `cargo check`, not clippy: the lanes already lint these packages, and the
# property here is whether each combination *resolves*.
set -euo pipefail

cd "$(dirname "$0")/.."

PACKAGES=(toolu-orm-connection toolu-orm-query toolu-orm-cli toolu-orm)

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

# "<orm-connection driver> <extra driver on orm-core>" — all six ordered pairs.
SUPERSETS=(
  "libsql postgres"
  "libsql rusqlite"
  "rusqlite postgres"
  "rusqlite libsql"
  "postgres libsql"
  "postgres rusqlite"
)

failed=()
checked=0

for pkg in "${PACKAGES[@]}"; do
  echo "$pkg"
  for combo in "${COMBOS[@]}"; do
    if [[ -z $combo && $pkg == toolu-orm-cli ]]; then
      printf '  %-26s skipped (needs a driver; see header)\n' "(no driver)"
      continue
    fi
    label="${combo:-(no driver)}"
    # --no-default-features so orm-core gets exactly this set; its default is
    # libsql, which would otherwise contaminate every combination.
    args=(check -q -p "$pkg" --no-default-features)
    [[ -n $combo ]] && args+=(--features "$combo")

    printf '  %-26s ' "$label"
    checked=$((checked + 1))
    if log=$(cargo "${args[@]}" 2>&1); then
      echo "ok"
    else
      echo "FAILED"
      printf '%s\n' "$log" | sed 's/^/      /'
      failed+=("$pkg $label")
    fi
  done
done

echo "toolu-orm-connection with a wider toolu-orm-core"
for pair in "${SUPERSETS[@]}"; do
  read -r own extra <<<"$pair"
  label="$own + core $extra"
  printf '  %-26s ' "$label"
  checked=$((checked + 1))
  if log=$(cargo check -q -p toolu-orm-connection -p toolu-orm-core \
    --no-default-features \
    --features "toolu-orm-connection/$own,toolu-orm-core/$extra" 2>&1); then
    echo "ok"
  else
    echo "FAILED"
    printf '%s\n' "$log" | sed 's/^/      /'
    failed+=("toolu-orm-connection $label")
  fi
done

if ((${#failed[@]})); then
  printf 'driver-matrix: %d combination(s) failed:\n' "${#failed[@]}" >&2
  printf '  %s\n' "${failed[@]}" >&2
  exit 1
fi

echo "driver-matrix: the driver-dependent crates compile on all ${#COMBOS[@]}" \
  "driver combinations (${checked} builds, including ${#SUPERSETS[@]} where" \
  "toolu-orm-core carries a driver its dependent does not)"
