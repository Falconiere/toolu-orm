# FromRow derive

**Feature:** `#[derive(FromRow)]` maps a row into a struct positionally (each field read at its own index, in declaration order) and exposes `REQUIRED_COLUMNS`, so `select_for::<T>()` selects exactly the columns `T` needs, in the order it decodes them.
**Drivers:** all of them, in whatever shape `toolu-orm-core`'s unified features gave the trait. One driver yields `from_row`; two or more yield one real decoder per driver. The derive reaches that shape through `toolu_orm_core::impl_derived_from_row!`, whose eight `#[cfg]`-gated definitions are compiled with `toolu-orm-core`'s own features (`crates/orm-core/src/row/derived.rs`), which is what lets a derive expanded in a consumer crate follow features it cannot see.
**Spec:** AC-18, plus `docs/toolu/specs/2026-09-07-fromrow-derive-driver-set-design.md` ([#17](https://github.com/Falconiere/toolu-orm/issues/17)).

## What is proven

- **libsql alone** (`from_row(&libsql::Row)`): a four-field `User` decodes two real in-memory libsql rows, `NULL` age into `None`.
- **rusqlite alone** (`from_row(&rusqlite::Row<'_>)`): the same struct decodes the same two rows from a real in-memory rusqlite database.
- **postgres + libsql** (the two-driver shape): one derived `Person` decodes real Postgres rows *and* real libsql rows. This replaced the old `from_libsql_row` error stub — the derive no longer has a method that only returns `"<Type> is only decoded from Postgres rows"`.
- **Missing columns:** selecting fewer columns than the struct declares fails with `RowMapping` naming the first missing index and column (`column 2 (email)`), not a panic — asserted on both single-driver lanes with raw driver SQL, since `select_for::<T>()` always selects `REQUIRED_COLUMNS`.
- **A documented driver difference:** an absent *trailing nullable* column decodes as `None` on libsql, which passes SQLite's `sqlite3_column_value` behavior through and reads an out-of-range index as SQL `NULL`, but is rejected on rusqlite, which validates the index against the statement. Both are pinned so the asymmetry is a known driver trait rather than a suspected derive bug.
- **`#[from_row(with = "f")]`:** normalizes an accepted value and surfaces `f`'s rejection as `RowMapping` naming that column, on both single-driver lanes.
- **Every existing single-driver suite** now decodes through the derive: the libsql and rusqlite `users` fixtures dropped their hand-written impls, so the mutations, reads and relational binaries on both lanes exercise it end to end.
- **A facade-only consumer** derives it with `toolu-orm` as its single dependency, on the default (libsql-only) lane — the derive's macro call and row type both resolve through `::toolu_orm::core::…`.
- **All eight driver combinations compile**, in two packages, guarded by `scripts/check-derive-matrix.sh`. `toolu-orm-facade-consumer` is the decisive one: `toolu-orm` is its only dependency, so in the rusqlite-only build `tokio-postgres` is absent from its dependency graph entirely (`cargo tree` finds no occurrence), yet the derive — which names `tokio_postgres::Row` in its postgres decoder unconditionally — still compiles. That is what pins the claim that an inactive driver's decoder costs nothing: its tokens are bound to a `$…:block` the surviving macro arm never interpolates, so they are dropped before name resolution rather than resolved and discarded. `toolu-orm-macros` covers the wider derive surface (`#[from_row(with)]`, renamed columns, several structs) but cannot prove absence, since its dev-dependencies pull in all three driver crates.

## How to run

Each of the eight combinations, including the no-driver arm. The four CI lanes
give `toolu-orm-core` only four of them, so this guard is what keeps the other
four definitions of `impl_derived_from_row!` honest — it runs in CI and in the
quality gate, and it fails naming the combination that broke:

```sh
bash scripts/check-derive-matrix.sh
```

The suites themselves:

```sh
# single-driver lanes
cargo nextest run -p toolu-orm-query --features libsql   -E 'binary(libsql_derived_from_row_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_derived_from_row_test)'

# default lane: derive output and the facade-only consumer, orm-core on libsql alone
cargo nextest run --workspace -E 'binary(from_row_test) | binary(facade_only_from_row_test)'

# two-driver shape, against live Postgres
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(from_row_derive_live_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| postgres | from_row_derive_live_test | derive_decodes_postgres_rows_with_null_as_none |
| postgres | from_row_derive_live_test | derive_fewer_columns_than_required_is_row_mapping |
| postgres | from_row_derive_live_test | derive_decodes_libsql_rows_with_null_as_none |
| default | from_row_test | test_from_row_generates_impl_for_named_struct |
| default | from_row_test | from_row_generates_required_columns |
| libsql-only | libsql_derived_from_row_test | derive_decodes_real_libsql_rows_with_null_as_none |
| libsql-only | libsql_derived_from_row_test | derive_reports_required_columns_in_field_order |
| libsql-only | libsql_derived_from_row_test | derive_on_a_short_select_is_row_mapping_naming_the_column |
| libsql-only | libsql_derived_from_row_test | derive_absent_trailing_nullable_column_decodes_as_none |
| libsql-only | libsql_derived_from_row_test | derive_with_attribute_normalizes_then_rejects |
| rusqlite-only | rusqlite_derived_from_row_test | derive_decodes_real_rusqlite_rows_with_null_as_none |
| rusqlite-only | rusqlite_derived_from_row_test | derive_reports_required_columns_in_field_order |
| rusqlite-only | rusqlite_derived_from_row_test | derive_on_a_short_select_is_row_mapping_naming_the_column |
| rusqlite-only | rusqlite_derived_from_row_test | derive_absent_trailing_nullable_column_is_rejected |
| rusqlite-only | rusqlite_derived_from_row_test | derive_with_attribute_normalizes_then_rejects |
