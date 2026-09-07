# Blocking connection

**Why it exists.** `DbConnection` is async, and rusqlite is not. Until now the rusqlite driver satisfied the async contract by wrapping every statement in `tokio::task::spawn_blocking`, which made a tokio runtime a hard requirement for consumers that have no async of their own (a CLI paying runtime startup per invocation) and added a second thread hop for consumers that already offload (an axum handler running its store work inside its own `spawn_blocking`).

`DbConnectionBlocking` is the synchronous sibling, implemented **natively** for `RusqliteConnection` — take the lock, call rusqlite, return. libsql and Postgres do not implement it: they talk to a server and are genuinely async, so a blocking wrapper there would only hide a `block_on`.

Paired with `RusqliteConnection::from_connection` (sync, non-fallible), there is now a complete path from a `rusqlite::Connection` to decoded rows with no runtime anywhere.

## What these suites prove

- **No runtime.** `blocking_roundtrip_without_a_runtime` is a plain `#[test]`: DDL, an insert returning `1`, and a decoded `SELECT` on an in-memory database. A `spawn_blocking` left anywhere on the path would fail it with "there is no reactor running".
- **No `Send` bound.** `query_map_accepts_a_non_send_row` decodes into a struct holding an `Rc<String>`. The async `query_map` requires `T: Send + 'static`; the blocking one decodes on the caller's thread and does not.
- **Errors, not panics.** `bad_sql_returns_a_query_error` gets `DbError::Query` naming the missing table, the same variant the async path returns.
- **Decode failures are mapping failures.** `decode_failure_returns_a_row_mapping_error` gets `DbError::RowMapping` carrying the `FromRow` message, matching the libsql backend and the documented `DbConnection` contract. Rows are decoded in the driver rather than in rusqlite's `query_map` callback, which would have needed a column index and column type this layer does not know.
- **A panicking task is a connection failure.** `async_reports_a_panicking_task_as_a_connection_error` panics inside the delegated blocking task; the resulting `JoinError` says nothing about the statement, so the async path reports `DbError::Connection` rather than a query failure.
- **The two paths cannot drift.** The async `DbConnection` impl delegates to the blocking methods inside `spawn_blocking`. `async_and_blocking_share_one_connection` writes through one path and reads through the other, in both directions, on one connection.
- **Callable from inside a runtime.** `blocking_calls_work_inside_a_runtime` runs the blocking methods inside `spawn_blocking` on a multi-thread runtime — the "already offloads" consumer. This is the case that pins the connection's `std::sync::Mutex`: `tokio::sync::Mutex::blocking_lock` panics in an async execution context.
- **Contention.** `concurrent_threads_serialize_on_the_connection` shares `&RusqliteConnection` across two OS threads writing 50 rows each; all 100 land.
- **Exported at the crate root.** Both suites import `toolu_orm_connection::DbConnectionBlocking` and call every method on it, so the trait being unreachable from a consumer is a compile failure in the lane, with no separate importability test to assert nothing at runtime.
- **Poisoning.** `a_poisoned_connection_reports_it` unwinds a panicking `FromRow` out of `query_map` while the lock is held, then asserts that both the blocking and the async path refuse the connection with `DbError::Connection` rather than handing out a connection that may be stuck mid-transaction.

## Tests

| Lane | Binary | Test |
|---|---|---|
| rusqlite-only | rusqlite_blocking_test | blocking_roundtrip_without_a_runtime |
| rusqlite-only | rusqlite_blocking_test | query_map_accepts_a_non_send_row |
| rusqlite-only | rusqlite_blocking_test | bad_sql_returns_a_query_error |
| rusqlite-only | rusqlite_blocking_test | decode_failure_returns_a_row_mapping_error |
| rusqlite-only | rusqlite_blocking_test | async_and_blocking_share_one_connection |
| rusqlite-only | rusqlite_blocking_concurrency_test | blocking_calls_work_inside_a_runtime |
| rusqlite-only | rusqlite_blocking_concurrency_test | concurrent_threads_serialize_on_the_connection |
| rusqlite-only | rusqlite_blocking_concurrency_test | async_reports_a_panicking_task_as_a_connection_error |
| rusqlite-only | rusqlite_blocking_concurrency_test | a_poisoned_connection_reports_it |
