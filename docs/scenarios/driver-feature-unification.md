# Driver feature unification

**Feature:** only `toolu-orm-core` decides which `FromRow` method exists. Every other crate decodes through `toolu_orm_core::row::from_postgres_row` / `from_libsql_row` / `from_rusqlite_row`, so no crate has to guess what Cargo unified onto orm-core.
**Drivers:** the supported feature combinations described below.
**Spec:** [#124](https://github.com/Falconiere/toolu-orm/issues/124).

The extra-core-driver checks cover **orm-connection**, not arbitrary query/core
feature mismatches. Query's libsql and rusqlite scalar decoders still implement
the single-driver `FromRow` shape, so keep core and query aligned when using
either SQLite executor.

## The problem this closes

`FromRow` changes shape with orm-core's *unified* feature set: one driver gives
`from_row`, two or more give `from_pg_row` / `from_libsql_row` /
`from_rusqlite_row`. A crate that picks a method from its **own** feature flags
is using a proxy for that set, and the proxy drifts as soon as one crate
forwards a driver feature to orm-core but not to its sibling.

That is exactly what `crates/orm-query/Cargo.toml` did: `rusqlite` forwarded to
`toolu-orm-connection/rusqlite`, `libsql` forwarded only to
`toolu-orm-core/libsql`. With both features on, orm-connection compiled
believing it was rusqlite-only and called `T::from_row`, while orm-core had two
drivers and exposed `from_rusqlite_row`. On v0.8.0:

```
$ cargo check -p toolu-orm-query --features rusqlite,libsql
error[E0599]: no function or associated item named `from_row` found for type parameter `T`
  --> crates/orm-connection/src/rusqlite_impl/blocking.rs:57:23
```

The Postgres path never had this bug, because `from_postgres_row` already
existed and orm-query's Postgres executor called it instead of naming a method.
The fix is the two siblings that were never written, plus the call sites that
now use all three.

## What is proven

- **The helper forwards to the right method on the multi-driver shape.** In the
  postgres lane orm-core is unified to `postgres` + `libsql`, so `FromRow` has
  no `from_row` at all. `from_libsql_row` decodes real in-memory libsql rows
  there, `NULL` age into `None`.
- **And on the single-driver shape.** In the rusqlite-only lane orm-core carries
  `rusqlite` alone, so `FromRow` exposes `from_row`; the same helper call
  decodes real in-memory rusqlite rows, and the call site never changed.
- **Decode failures pass through unchanged.** A projection missing a required
  column is `DbCoreError::RowMapping` naming the index and column
  (`column 1 (age)`) on rusqlite — the helper adds no validation and swallows
  nothing.
- **The documented driver difference survives the indirection.** The same
  narrowed projection decodes as `None` on libsql, which reads an out-of-range
  index as SQL `NULL`, rather than failing. Both halves are pinned here for the
  same reason they are pinned in [FromRow derive](from-row-derive.md): so the
  asymmetry stays a known driver trait instead of a suspected helper bug.
- **All eight driver combinations compile, for the crates that decode.**
  `scripts/check-driver-matrix.sh` builds orm-connection, orm-query, orm-cli and
  the `toolu-orm` facade against every subset of `{postgres, libsql, rusqlite}`.
  orm-cli skips the no-driver subset because its migration decoders require a driver.
  The three `toolu-orm-query` combinations that fail on v0.8.0 —
  `postgres,rusqlite`, `libsql,rusqlite`, `postgres,libsql,rusqlite` — are the
  regression guard.
- **Including when orm-core is strictly wider than its dependent.** Six further
  cases build orm-connection on one driver while orm-core additionally carries
  another. No own-feature combination produces that for `libsql_impl.rs`, so
  without them its decoder would go unchecked against a multi-driver orm-core;
  on v0.8.0 the `libsql + core postgres` case fails at `libsql_impl.rs:155`.

## Scope

The matrix's superset phase covers orm-connection only.
`crates/orm-query/src/select/executor_fetch/` still hand-writes `FromRow` for a
count scalar in the single-driver shape. A consumer reaches that only by giving
orm-core a driver it withholds from orm-query, which contradicts "consumers
activate the drivers they need on every crate they depend on" and which no
manifest in this repo produces. `scripts/check-driver-matrix.sh` says so in its
header rather than leaving the boundary implicit.

`toolu-orm-cli` skips the no-driver cell. Its migration runner decodes rows
unconditionally and the zero-driver `FromRow` partition has no methods, so
`PragmaInt` and `AppliedMigration` cannot implement it — orm-cli genuinely
requires a driver, which its `default = ["libsql"]` already states. That
predates this scenario.

## How to run

The guard, which runs in CI and in the quality gate and fails naming the
combination that broke:

```sh
bash scripts/check-driver-matrix.sh
```

The suites:

```sh
# multi-driver shape: orm-core unified to postgres + libsql
docker compose -f docker-compose.test.yaml up -d --wait
cargo nextest run -p toolu-orm-connection --features libsql,postgres -E 'binary(libsql_core_decode_test)'

# single-driver shape: orm-core on rusqlite alone
cargo nextest run -p toolu-orm-connection --features rusqlite,sqlite-vec -E 'binary(rusqlite_core_decode_test)'
```

`--features postgres` alone would leave orm-connection without `libsql` and the
first binary would list zero tests. In CI it runs inside the postgres lane,
where `toolu-orm-cli`'s default `libsql` feature supplies
`toolu-orm-connection/libsql`.

## Tests

| Lane | Binary | Test |
|---|---|---|
| postgres | libsql_core_decode_test | from_libsql_row_decodes_real_rows_with_null_as_none |
| postgres | libsql_core_decode_test | from_libsql_row_absent_column_decodes_as_none |
| rusqlite-only | rusqlite_core_decode_test | from_rusqlite_row_decodes_real_rows_with_null_as_none |
| rusqlite-only | rusqlite_core_decode_test | from_rusqlite_row_missing_column_is_row_mapping |
