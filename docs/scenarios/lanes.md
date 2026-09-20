# Lanes and revived suites

**Why lanes exist.** `FromRow` changes shape with the driver features unified on orm-core (one driver: `from_row`; two or more: `from_pg_row` / `from_libsql_row` / `from_rusqlite_row`), and orm-query compiles its executor, transaction, and fetch code only when exactly one driver is active (`cfg_single_backend!`). A test target is real only if some CI lane satisfies its `required-features`.

| Lane | Command | Compiles |
|---|---|---|
| default | `cargo nextest run --workspace` | orm-core and orm-cli with libsql, orm-query with no driver (SQL-generation tests only), the `toolu-orm` facade with no driver |
| postgres | The seven-package Postgres command in [the quality gate](../../CLAUDE.md#quality-gate) | orm-core postgres+libsql (the two-driver derive shape), orm-query postgres alone, every live-Postgres suite and the facade-only consumer |
| libsql-only | `cargo nextest run -p toolu-orm-query --features libsql` | orm-query's libsql executor, `run_transaction`, fetch methods |
| rusqlite-only | `cargo nextest run -p toolu-orm-query --features rusqlite,sqlite-vec` and `-p toolu-orm-connection --features rusqlite,sqlite-vec` and `-p toolu-orm-cli --no-default-features --features rusqlite` | orm-query's sync rusqlite executor (raw + `RusqliteConnection`), live sqlite-vec, `DbConnectionBlocking`, and blocking migrate/status/baseline twins (see [Blocking connection](blocking-connection.md)) |

Between them these four lanes give orm-core only four of the eight driver
combinations — libsql, postgres+libsql, rusqlite and postgres — so the other
four shapes of `FromRow` are never exercised by a lane. Two compile-only checks
cover the gap:

- `bash scripts/check-derive-matrix.sh` compiles `#[derive(FromRow)]` against all
  eight (see [FromRow derive](from-row-derive.md)).
- `bash scripts/check-driver-matrix.sh` compiles the driver-dependent *crates* —
  orm-connection, orm-query, orm-cli and the `toolu-orm` facade — against all
  eight (except orm-cli with no driver), plus six cases where orm-core carries
  a driver orm-connection does not
  (see [Driver feature unification](driver-feature-unification.md)). The derive
  matrix never builds those crates, which is how issue #124 shipped a
  `toolu-orm-query --features rusqlite,libsql` that did not compile.

Before this program, only the first two lanes ran. The four suites below existed but could not compile (`E0407: method from_pg_row is not a member of trait FromRow`): they implemented the two-driver shape while their `required-features` resolved to one driver. They now implement `from_row` and run on their lane.

A suite can also be missing for a reason no lane explains: `query_column_test` was never *built*, because its entry file was `mod.rs` rather than `main.rs` and `orm-core` names no `[[test]]` path (issue #125, 33 tests inert). `bash scripts/check-test-targets.sh` closes that gap the way `check-derive-matrix.sh` closes the feature-combination one; the revived tests are documented in [Expression fragments](expr-fragments.md).

## What the revived suites prove

- **executor_test (libsql):** `InsertBuilder` / `SelectBuilder` / `UpdateBuilder` / `DeleteBuilder` executed on an in-memory libsql database; `fetch_one` on an empty table is `QueryError::NotFound`; `count` matches inserted rows.
- **integration_test (libsql):** the `#[table]`-generated `select()` / `insert()` / `update()` / `delete()` factories end to end, filters, order + pagination, `count` / `exists`, transactions, dynamic filter lists.
- **transaction_test (libsql):** see [Transactions](transactions.md).
- **rusqlite_impl_test (rusqlite):** `RusqliteConnection::open_in_memory()`: `execute_batch` DDL, `execute_sql` insert, `query_map` decode, bad SQL maps to `DbError::Query`. Also the two other constructors, both of which now route through `from_connection`: adopting a caller-built connection keeps its database and its connection-scoped pragmas, and `open` still writes durably to a file.

## Tests

| Lane | Binary | Test |
|---|---|---|
| libsql-only | executor_test | insert_and_select |
| libsql-only | executor_test | select_empty_returns_empty_vec |
| libsql-only | executor_test | fetch_one_returns_single |
| libsql-only | executor_test | fetch_one_empty_returns_not_found |
| libsql-only | executor_test | fetch_optional_found |
| libsql-only | executor_test | fetch_optional_not_found |
| libsql-only | executor_test | update_modifies_row |
| libsql-only | executor_test | delete_removes_row |
| libsql-only | executor_test | count_returns_correct_number |
| libsql-only | integration_test | select_queries::insert_via_builder_and_select_back |
| libsql-only | integration_test | select_queries::select_with_filter |
| libsql-only | integration_test | select_queries::select_with_order_and_pagination |
| libsql-only | integration_test | select_queries::fetch_optional_found_and_not_found |
| libsql-only | integration_test | select_queries::fetch_one_not_found_returns_error |
| libsql-only | integration_test | mutation_queries::update_via_builder |
| libsql-only | integration_test | mutation_queries::delete_via_builder |
| libsql-only | integration_test | mutation_queries::count_and_exists |
| libsql-only | integration_test | mutation_queries::transaction_commit |
| libsql-only | integration_test | mutation_queries::transaction_rollback |
| libsql-only | integration_test | mutation_queries::dynamic_filters |
| libsql-only | transaction_test | transaction_commit_on_ok |
| libsql-only | transaction_test | transaction_rollback_on_err |
| libsql-only | transaction_test | transaction_multiple_operations |
| rusqlite-only | rusqlite_impl_test | execute_batch_creates_table |
| rusqlite-only | rusqlite_impl_test | execute_sql_inserts_row |
| rusqlite-only | rusqlite_impl_test | query_map_returns_rows |
| rusqlite-only | rusqlite_impl_test | execute_sql_returns_error_on_bad_sql |
| rusqlite-only | rusqlite_impl_test | from_connection_adopts_existing_database |
| rusqlite-only | rusqlite_impl_test | from_connection_preserves_connection_pragmas |
| rusqlite-only | rusqlite_impl_test | open_persists_to_a_file |
