# Prepared-statement cache

**Feature:** Both rusqlite adapters -- `Executor for rusqlite::Connection` (`crates/orm-query/src/executor/rusqlite_impl.rs`) and `DbConnectionBlocking for RusqliteConnection` (`crates/orm-connection/src/rusqlite_impl.rs`) -- reuse the connection's bounded statement cache via `Connection::prepare_cached` instead of re-parsing SQL text with `prepare`/`execute` on every call.
**Drivers:** rusqlite only.
**Spec:** `docs/toolu/specs/2026-09-16-rusqlite-prepared-statement-cache-design.md`, AC-1 through AC-6.

## What is proven

| Scenario | Table state | Result |
|---|---|---|
| Same SQL text, changed parameters | two rows | each call returns the row matching its own parameter; no deadlock on the second call |
| Unrelated schema change between two calls to the same SQL text | one row, then an unrelated table is created | the second call's result is unchanged from the first |
| The queried table is dropped between two calls to the same SQL text | one row, then the table is dropped | the second call returns a query error (`QueryError::Driver` / `DbError::Query`), not a panic or a stale success |
| A schema change to the queried table that leaves referenced columns untouched (`ALTER TABLE ... ADD COLUMN`) | one row, then a column is added | the second call's result is unchanged from the first |
| A schema change that invalidates a column the cached SQL text references (`ALTER TABLE ... RENAME COLUMN`) | one row, then a referenced column is renamed | the second call returns a query error, not stale or silently wrong data |
| A failing query, then reuse of SQL text that already succeeded | one row | the earlier failure does not poison the connection; the reused SQL text still succeeds with the same result |
| A `FromRow` decode failure through a cached statement | one row, wrong column type requested | `QueryError::RowMapping` / `DbError::RowMapping`, unchanged from the uncached path |
| `execute_sql` (write path), same SQL text, changed parameters | empty, then two inserts | both rows land with the correct data and `affected == 1` each time |

Every read test calls twice on the same connection right after each other,
which is also the proof that the `MutexGuard` (`RusqliteConnection`) and the
`CachedStatement` borrow are both released at the end of each call: a leaked
borrow would hang the second call rather than return a wrong answer.

The two schema-change tests pin a property the adapters themselves do nothing
to provide: SQLite revalidates a cached statement's schema cookie on every
execution and recompiles it against the current schema before running, at the
C-library level, regardless of whether the statement handle came from
`prepare` or `prepare_cached`. Caching therefore adds no staleness risk beyond
what the uncached path already had -- a schema change either leaves the
cached statement's result unchanged (`ADD COLUMN`, an untouched column) or
surfaces as an ordinary query error on next use (`RENAME COLUMN`, a
referenced column), never stale or silently wrong data. See the doc comments
on `Executor for rusqlite::Connection` (`crates/orm-query/src/executor/rusqlite_impl.rs`)
and `DbConnectionBlocking for RusqliteConnection` (`crates/orm-connection/src/rusqlite_impl.rs`).

A release-mode benchmark, `crates/orm-connection/examples/rusqlite_prepared_statement_bench.rs`
(`cargo run --release --example rusqlite_prepared_statement_bench -p toolu-orm-connection --features rusqlite`),
compares the uncached and cached paths on a bare `rusqlite::Connection`,
isolates preparation-only cost from row decoding, and compares the blocking
`RusqliteConnection` wrapper against its async `spawn_blocking` path, with a
checksum equality assertion across all four data-producing variants.

## How to run

```sh
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_prepared_statement_cache_test)'
cargo nextest run -p toolu-orm-connection --features rusqlite -E 'binary(rusqlite_prepared_statement_cache_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| rusqlite-only | rusqlite_prepared_statement_cache_test | reads::query_map_reuses_cached_statement_across_changed_parameters |
| rusqlite-only | rusqlite_prepared_statement_cache_test | reads::query_map_reuse_survives_unrelated_schema_change |
| rusqlite-only | rusqlite_prepared_statement_cache_test | reads::query_map_reuse_reports_error_after_table_drop |
| rusqlite-only | rusqlite_prepared_statement_cache_test | reads::query_map_reuse_survives_added_column_on_the_queried_table |
| rusqlite-only | rusqlite_prepared_statement_cache_test | reads::query_map_reuse_errors_after_a_referenced_column_is_renamed |
| rusqlite-only | rusqlite_prepared_statement_cache_test | reads::query_map_error_does_not_poison_later_reuse |
| rusqlite-only | rusqlite_prepared_statement_cache_test | decode_failure::query_map_decode_failure_is_row_mapping_error |
| rusqlite-only | rusqlite_prepared_statement_cache_test | writes::execute_sql_reuses_cached_statement_across_changed_parameters |
