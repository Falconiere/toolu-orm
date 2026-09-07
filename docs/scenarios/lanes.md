# Lanes and revived suites

**Why lanes exist.** `FromRow` changes shape with the driver features unified on orm-core (one driver: `from_row`; two or more: `from_pg_row` / `from_libsql_row` / `from_rusqlite_row`), and orm-query compiles its executor, transaction, and fetch code only when exactly one driver is active (`cfg_single_backend!`). A test target is real only if some CI lane satisfies its `required-features`.

| Lane | Command | Compiles |
|---|---|---|
| default | `cargo nextest run --workspace` | orm-core and orm-cli with libsql, orm-query with no driver (SQL-generation tests only) |
| postgres | `cargo nextest run -p <five crates> --features postgres` | orm-core postgres+libsql (the derive shape), orm-query postgres alone, every live-Postgres suite |
| libsql-only | `cargo nextest run -p toolu-orm-query --features libsql` | orm-query's libsql executor, `run_transaction`, fetch methods |
| rusqlite-only | `cargo nextest run -p toolu-orm-query --features rusqlite` and `-p toolu-orm-connection --features rusqlite` | orm-query's sync rusqlite executor and the rusqlite `DbConnection` |

Before this program, only the first two lanes ran. The four suites below existed but could not compile (`E0407: method from_pg_row is not a member of trait FromRow`): they implemented the two-driver shape while their `required-features` resolved to one driver. They now implement `from_row` and run on their lane.

## What the revived suites prove

- **executor_test (libsql):** `InsertBuilder` / `SelectBuilder` / `UpdateBuilder` / `DeleteBuilder` executed on an in-memory libsql database; `fetch_one` on an empty table is `QueryError::NotFound`; `count` matches inserted rows.
- **integration_test (libsql):** the `#[table]`-generated `select()` / `insert()` / `update()` / `delete()` factories end to end, filters, order + pagination, `count` / `exists`, transactions, dynamic filter lists.
- **transaction_test (libsql):** see [Transactions](transactions.md).
- **rusqlite_impl_test (rusqlite):** `RusqliteConnection::open_in_memory()`: `execute_batch` DDL, `execute_sql` insert, `query_map` decode, bad SQL maps to `DbError::Query`.

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
