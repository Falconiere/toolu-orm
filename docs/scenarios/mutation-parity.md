# Mutation parity

**Feature:** the SQLite mutation clauses Drizzle exposes that stock SQLite actually runs, spelled the same way on Postgres.

- `UpdateBuilder::returning` / `DeleteBuilder::returning` append `RETURNING`, read back with `fetch_one` / `fetch_optional` / `fetch_all`. The clause binds nothing. A filter that matches nothing makes `fetch_optional` `None` and `fetch_one` `QueryError::NotFound`. On rusqlite, `execute` of a `RETURNING` statement is a driver error (`Execute returned results`); use a fetch method.
- `OnConflict::where_target` is `ON CONFLICT (…) WHERE …`, the predicate that names a partial unique index. It has to be that index's own expression (`deleted = 0`, not a bound stand-in for `0`). It binds after `VALUES` and before the `DO UPDATE` assignments, because that is where SQL puts it.
- `OnConflict::where_update` is `DO UPDATE SET … WHERE …`. A false guard skips the write: nothing changes and `RETURNING` projects no row. It binds after the assignments. `do_nothing` discards it, and a clause that never records an assignment does not render it.

**Not shipped:** `ORDER BY` and `LIMIT` on `UPDATE` and `DELETE`. Drizzle documents them, but SQLite accepts them only when built with `SQLITE_ENABLE_UPDATE_DELETE_LIMIT`. Bundled rusqlite reports that option unused, and both statements are syntax errors. See `bundled_sqlite_rejects_order_by_and_limit_on_update_and_delete`.

**Drivers:** rusqlite, Postgres. libsql is not a separate lane for this page; the SQL is the same SQLite text rusqlite runs.

## What is proven

- An update `RETURNING` hands back the written value; a delete `RETURNING` hands back the removed row.
- `fetch_one` on a filter that matches nothing is `NotFound` carrying the table name, and the stored row is unchanged.
- A `DO UPDATE … WHERE` guard leaves a non-matching row alone and returns nothing, then writes when the same guard holds.
- `ON CONFLICT (sku) WHERE deleted = 0` does not treat a `deleted = 1` row as a conflict, then updates the live row on the next insert and leaves the tombstone.
- A returned update inside a transaction is gone after rollback (rusqlite `ROLLBACK`, Postgres `PgTransaction::rollback`).
- Placeholder order is `VALUES`, then the index predicate, then the assignments, then the update guard, with `$N` on Postgres.

## How to run

```sh
cargo nextest run -p toolu-orm-query -E 'binary(mutation_parity_sql_test)'
cargo nextest run -p toolu-orm-query --features rusqlite -E 'binary(rusqlite_mutation_parity_test)'
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-query --features postgres -E 'binary(postgres_mutation_parity_test)'
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| default | mutation_parity_sql_test | an_index_predicate_binds_before_the_assignments_and_the_guard_after |
| default | mutation_parity_sql_test | an_update_guard_without_an_assignment_binds_nothing |
| default | mutation_parity_sql_test | do_nothing_discards_the_update_guard_and_keeps_the_index_predicate |
| default | mutation_parity_sql_test | no_returning_call_leaves_update_and_delete_unchanged |
| default | mutation_parity_sql_test | returning_projects_delete_columns_after_the_where_clause |
| default | mutation_parity_sql_test | returning_projects_update_columns_after_the_where_clause |
| default | mutation_parity_sql_test | the_same_clauses_number_dollar_placeholders_on_postgres |
| rusqlite-only | rusqlite_mutation_parity_test | conflict::an_index_predicate_misses_a_row_the_partial_index_excludes |
| rusqlite-only | rusqlite_mutation_parity_test | conflict::an_update_guard_skips_the_row_when_it_fails |
| rusqlite-only | rusqlite_mutation_parity_test | limit::bundled_sqlite_rejects_order_by_and_limit_on_update_and_delete |
| rusqlite-only | rusqlite_mutation_parity_test | returning::delete_returning_hands_back_the_removed_row |
| rusqlite-only | rusqlite_mutation_parity_test | returning::execute_refuses_a_returning_update_on_rusqlite |
| rusqlite-only | rusqlite_mutation_parity_test | returning::fetch_one_reports_not_found_when_nothing_matched |
| rusqlite-only | rusqlite_mutation_parity_test | returning::rollback_discards_the_returned_update |
| rusqlite-only | rusqlite_mutation_parity_test | returning::update_returning_hands_back_the_written_row |
| postgres | postgres_mutation_parity_test | an_index_predicate_misses_a_row_the_partial_index_excludes |
| postgres | postgres_mutation_parity_test | an_update_guard_skips_the_row_when_it_fails |
| postgres | postgres_mutation_parity_test | delete_returning_hands_back_the_removed_row |
| postgres | postgres_mutation_parity_test | fetch_one_reports_not_found_when_nothing_matched |
| postgres | postgres_mutation_parity_test | rollback_discards_the_returned_update |
| postgres | postgres_mutation_parity_test | update_returning_hands_back_the_written_row |
