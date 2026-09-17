# Postgres connection

**Feature:** `PgDatabase::init(&PgConfig)` opens a `deadpool-postgres` pool with an eager connectivity check; `connect()` hands out a `PgConnection` implementing `DbConnection` (`execute_sql`, `query_map`, `execute_batch`). `PgConfig::checkout_timeout` bounds how long `connect()` waits for a pool slot. `execute_sql` and `query_map` prepare through the connection's deadpool statement cache instead of re-preparing the SQL on every call; `PgDatabase::clear_statement_caches()` empties those caches.
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
- Repeated SQL with different bind values is prepared once per connection: after five `query_map` reads and five `execute_sql` writes, `pg_prepared_statements` holds exactly one row for each SQL text with its `generic_plans + custom_plans` counter at 5, and every row and affected-row count is still correct.
- A `PgTransaction` shares the connection's cache: SQL first run inside a transaction and then, after `commit()`, on the connection stays one prepared statement carrying the executions from both sides.
- A transaction dropped without commit rolls its write back and leaves its statement usable — a protocol-level `Parse` is not undone by `ROLLBACK`.
- Any statement the server rejects is evicted, not retried: a duplicate primary key returns `23505`, leaves zero prepared statements for that SQL, and the next `INSERT` of a different row re-prepares and succeeds — with the table holding two rows, not three, so the failed write was never replayed.
- DDL that invalidates a cached plan surfaces `0A000` once: the failing call evicts the statement (`pg_prepared_statements` drops to zero rows for that text) and the next call re-prepares and reads the widened row. Nothing is retried on the caller's behalf.
- `PgDatabase::clear_statement_caches()` closes the cached statements of every connection the pool handed out; the next call re-prepares, with the execution counter restarting at 1.
- `checkout_timeout: None` is an explicit opt-out — `connect()` keeps waiting past what a configured timeout would have allowed, matching pre-timeout behavior; `PgConfig::DEFAULT_CHECKOUT_TIMEOUT` (5s) is the documented default used by `PgConfig::for_test()`. Both the TLS and non-TLS pool builders apply the same `checkout_timeout`; only the non-TLS path runs live (no TLS Postgres fixture exists in this repo), the TLS builder's parity is a direct code-read of `postgres_impl/pool.rs`.

## Prepared-statement cache

`pg_execute_sql` / `pg_query_map` call `prepare_cached` and execute the returned
`Statement`. Handing tokio-postgres a `&str` instead parses, describes, executes
and closes a fresh statement every call.

- **Ownership and lifetime.** One `StatementCache` per physical connection,
  created by deadpool with the connection and dropped with it, which closes its
  server-side statements. A `PgTransaction` holds a clone of the connection's
  `Arc<StatementCache>`, so the same SQL costs one `Parse` whether it first runs
  inside or outside a transaction. A `Statement` is bound to the connection that
  prepared it and is never shared with another one.
- **Size.** Deadpool's cache never evicts, so N distinct SQL texts hold N
  prepared statements on that connection for its lifetime. Callers that render
  unbounded SQL text (for example `IN` lists of varying arity) should normalise
  it or call `PgDatabase::clear_statement_caches()`.
- **Failures.** A statement the server rejects is dropped from the cache and its
  error returned unchanged. Nothing is retried, so a non-idempotent write is
  never replayed after an uncertain outcome. Eviction is what keeps a cached
  plan that DDL invalidated (`0A000 cached plan must not change result type`)
  from failing that connection forever: the call after the failure re-prepares.
  DDL that does not change a statement's result type needs none of this —
  Postgres replans silently.
- **Transaction-pooling proxies.** Prepared statements are session state.
  PgBouncer in `transaction` or `statement` mode must track them (>= 1.21 with
  `max_prepared_statements > 0`), or a later `Bind` reaches a backend that never
  parsed the statement and answers `26000`. Session pooling and direct
  connections are unaffected.

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
| postgres | postgres_live_statement_cache_test | repeated_reads_prepare_the_statement_once |
| postgres | postgres_live_statement_cache_test | repeated_writes_prepare_the_statement_once |
| postgres | postgres_live_statement_cache_test | transaction_and_connection_share_one_cached_statement |
| postgres | postgres_live_statement_cache_test | dropped_transaction_rolls_back_and_leaves_the_statement_cached |
| postgres | postgres_live_statement_cache_test | a_rejected_statement_is_evicted_and_never_retried |
| postgres | postgres_live_statement_cache_test | ddl_invalidated_statement_is_evicted_then_re_prepared |
| postgres | postgres_live_statement_cache_test | clear_statement_caches_closes_the_cached_statements |
