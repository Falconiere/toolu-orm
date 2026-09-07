# Postgres connection

**Feature:** `PgDatabase::init(&PgConfig)` opens a `deadpool-postgres` pool with an eager connectivity check; `connect()` hands out a `PgConnection` implementing `DbConnection` (`execute_sql`, `query_map`, `execute_batch`).
**Drivers:** Postgres only (SQLite has no pool; see [Lanes](lanes.md)).
**Spec:** AC-3.

## What is proven

- A fresh pool connects, creates a table, inserts through `execute_sql` with `$N` parameters, and reads the row back through `query_map`.
- Two `connect()` calls are two sessions: a `SET search_path` on the first is invisible to the second (the second gets SQLSTATE `42P01` for the schema-scoped table).
- `PgDatabase::init` against a closed port fails fast with `DbError::Connection("initial connection failed: ...")`; no lazy surprise on the first query.
- Bad SQL maps to `DbError::Query` whose message starts with the severity and carries the SQLSTATE (`ERROR: ... (42P01)`).
- `execute_batch` runs every statement; a failing batch is one implicit transaction, so nothing from it survives.
- A type mismatch while decoding maps to `DbError::RowMapping` naming the column.
- Every `Value` variant binds through `to_pg_params`: `Integer(i64::MAX)`, `Text("héllo")`, `Real(1.5)`, `Blob([0,255,7])`, `Null`.

## How to run

```sh
docker compose -f docker-compose.test.yaml up -d --wait
TEST_DB_PORT=5434 cargo nextest run -p toolu-orm-core -p toolu-orm-macros -p toolu-orm-query -p toolu-orm-connection -p toolu-orm-cli --features postgres -E 'binary(/^postgres_live_.*_test$/)'
```

Each test owns a schema (`DROP SCHEMA IF EXISTS ... CASCADE; CREATE SCHEMA ...; SET search_path`), so the suite runs in parallel and reruns are clean. The server address comes from `TEST_DB_HOST`, `TEST_DB_PORT`, `TEST_DB_USER`, `TEST_DB_PASSWORD` (defaults `localhost`, `5433`, `toolu`, `toolu`).

## Tests

| Lane | Binary | Test |
|---|---|---|
| postgres | postgres_live_pool_test | init_connect_insert_and_read_back |
| postgres | postgres_live_pool_test | pool_hands_out_independent_sessions |
| postgres | postgres_live_pool_test | init_fails_fast_when_server_is_unreachable |
| postgres | postgres_live_queries_test | execute_sql_on_missing_table_is_query_error_with_sqlstate |
| postgres | postgres_live_queries_test | execute_batch_runs_every_statement |
| postgres | postgres_live_queries_test | execute_batch_failure_leaves_nothing_behind |
| postgres | postgres_live_queries_test | query_map_type_mismatch_is_row_mapping_error |
| postgres | postgres_live_queries_test | query_map_binds_every_value_variant |
