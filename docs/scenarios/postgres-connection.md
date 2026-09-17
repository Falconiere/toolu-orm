# Postgres connection

**Feature:** `PgDatabase::init(&PgConfig)` opens a `deadpool-postgres` pool with an eager connectivity check; `connect()` hands out a `PgConnection` implementing `DbConnection` (`execute_sql`, `query_map`, `execute_batch`). `PgConfig::checkout_timeout` bounds how long `connect()` waits for a pool slot.
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
- `PgConfig::checkout_timeout` bounds how long `connect()` waits for a free pool slot (deadpool's checkout **wait** timeout only, distinct from connection-creation/recycle timeouts, which this config does not expose): against a real one-slot pool with the slot held, a second `connect()` returns `DbError::Pool` naming the timeout once the configured bound elapses, and not before it or long after it.
- Releasing the held connection before the deadline lets a waiting `connect()` succeed instead of timing out.
- A timed-out waiter does not leak pool capacity: once the original connection is released, a later `connect()` still succeeds.
- `checkout_timeout: None` is an explicit opt-out — `connect()` keeps waiting past what a configured timeout would have allowed, matching pre-timeout behavior; `PgConfig::DEFAULT_CHECKOUT_TIMEOUT` (5s) is the documented default used by `PgConfig::for_test()`. Both the TLS and non-TLS pool builders apply the same `checkout_timeout`; only the non-TLS path runs live (no TLS Postgres fixture exists in this repo), the TLS builder's parity is a direct code-read of `postgres_impl/pool.rs`.

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
| postgres | postgres_live_pool_timeout_test | checkout_times_out_near_the_configured_bound |
| postgres | postgres_live_pool_timeout_test | release_before_deadline_lets_a_waiter_succeed |
| postgres | postgres_live_pool_timeout_test | timed_out_waiter_does_not_leak_pool_capacity |
| postgres | postgres_live_pool_timeout_test | checkout_timeout_none_disables_the_wait_bound |
| postgres | postgres_live_queries_test | execute_sql_on_missing_table_is_query_error_with_sqlstate |
| postgres | postgres_live_queries_test | execute_batch_runs_every_statement |
| postgres | postgres_live_queries_test | execute_batch_failure_leaves_nothing_behind |
| postgres | postgres_live_queries_test | query_map_type_mismatch_is_row_mapping_error |
| postgres | postgres_live_queries_test | query_map_binds_every_value_variant |
