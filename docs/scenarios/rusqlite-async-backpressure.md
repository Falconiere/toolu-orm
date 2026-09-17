# rusqlite async backpressure

**Feature:** `DbConnection for RusqliteConnection` (`crates/orm-connection/src/rusqlite_impl/asynchronous.rs`) waits for a shared one-permit admission gate **before** it calls `tokio::task::spawn_blocking`, and moves that owned permit into the blocking closure. `DbConnectionBlocking` is unchanged: a direct blocking caller takes the connection mutex with no gate.
**Drivers:** rusqlite only.
**Spec:** issue #90.

## Why

A `rusqlite::Connection` runs one statement at a time, behind a `std::sync::Mutex`.
Before this change every async call entered tokio's blocking pool first and only
then waited for that mutex, so `N` contending callers occupied `N` blocking
threads while `N - 1` of them did nothing. Tokio's blocking pool is finite and
shared with everything else in the process, so a contended connection could
starve unrelated blocking work: with `max_blocking_threads(2)`, one 500 ms
statement plus one outstanding second call left an unrelated
`spawn_blocking(|| 7)` unserved.

## The contract

- **Queue.** One permit per connection, shared by every internal handle
  (`handle()` clones both `Arc`s). A caller waits on its own task, so a queue of
  contending callers costs no blocking thread. Admission is FIFO — tokio's
  semaphore hands out permits in request order.
- **Blocking callers are not gated.** A synchronous function cannot await a
  permit, and it runs on the thread it is already on rather than taking one from
  a runtime's blocking pool. They contend on the connection mutex only, so a
  long queue of async callers cannot shut them out (and, the mutex being unfair,
  they may overtake a queued async caller).
- **Cancellation before admission.** Dropping the future while it is queued
  removes it from the queue. No `spawn_blocking`, no SQL, no permit consumed.
- **Cancellation after admission.** A started blocking task cannot be aborted,
  so the statement runs to completion and its result is discarded. Because the
  permit is *owned by the closure*, the gate stays closed until that statement
  ends: a cancelling caller cannot admit a replacement that would sit on the
  mutex holding a second blocking thread. Holding the permit in the async future
  instead would release it on cancellation — that mutation fails
  `cancelling_after_admission_keeps_the_gate_closed` while every other test in
  the suite still passes.
- **Errors are unchanged.** Row-mapping failures stay `DbError::RowMapping`, a
  poisoned lock stays `DbError::Connection`, and a panicking blocking task stays
  the `JoinError`-derived `DbError::Connection`. The permit is dropped while the
  closure unwinds, so a panic reopens the gate rather than wedging the
  connection.
- **Constructors are not gated.** `open` / `open_in_memory` create the
  connection, so there is nothing to serialize against yet.

## What is proven

| Scenario | Setup | Result |
|---|---|---|
| Unrelated blocking work under contention | `max_blocking_threads(2)`, a 500 ms statement holding the connection, a second async call outstanding | an unrelated `spawn_blocking` returning `7` completes inside 100 ms; before the gate it did not |
| Cancelling while queued | 500 ms holder, a second async `INSERT` dropped by a 50 ms timeout | the insert's row is absent afterwards, and the next async call still succeeds |
| Cancelling after admission | holder future polled once, confirmed started from inside the decode, then dropped | the canary `spawn_blocking` is still served inside 100 ms, and a replacement read returns the right row once the cancelled statement ends |
| Concurrent async writers | 4 tasks x 25 inserts on one `Arc<RusqliteConnection>` | 100 rows, 100 distinct labels — nothing lost or duplicated |
| A direct blocking caller alongside a queue | a 200 ms holder, two queued async inserts, one `std::thread` using `DbConnectionBlocking` | the blocking insert lands; all four rows are present |
| Decode failure on the async path | wrong column type requested | `DbError::RowMapping` carrying the `FromRow` message, and the connection still answers afterwards |
| Panicking decode | `FromRow` that panics mid-row | `DbError::Connection` ("blocking task did not complete"), and the next async call returns the poisoning report instead of hanging on a lost permit |

The hold comes from a `FromRow` whose decode signals a `Notify` and then sleeps.
Row decoding runs inside `query_map`'s `MutexGuard`, between `sqlite3_step`
calls on a live statement, so the connection, the statement and one blocking
thread are all genuinely occupied — the same state a slow SQL statement
produces, without adding rusqlite's `functions` feature to a published
dependency for a test's sake.

## Cost

The gate trades throughput on very small statements for a blocking pool that
stays free. Release benchmark
(`crates/orm-connection/examples/rusqlite_async_backpressure_bench.rs`), 16
concurrent tasks sharing one connection, `max_blocking_threads(4)`, Apple
silicon, SQLite 3.53.2:

| Workload | Variant | Elapsed | Calls/s | Blocking threads touched | Unrelated `spawn_blocking` wait |
|---|---|---|---|---|---|
| `lookup` (one indexed row, 80,000 calls) | ungated (pre-change) | 166.8 ms | 481,927 | 4 | 39.8 µs |
| `lookup` | gated | 553.8 ms | 144,665 | 2 | 22.8 µs |
| `scan` (1,000 rows, 1,600 calls) | ungated (pre-change) | 76.4 ms | 21,052 | 4 | 564.5 µs |
| `scan` | gated | 83.1 ms | 19,277 | 2 | 14.7 µs |

Read it as two regimes. When the statement is shorter than the thread handoff
(`lookup`, roughly 2 µs of SQL), the ungated path pipelines: four pool threads
sit on the mutex and hand it over directly, while the gate exposes one
wake-plus-`spawn_blocking` round trip per call — 3.3x fewer calls per second.
When the connection itself is the bottleneck (`scan`, roughly 0.6 ms of SQL),
throughput is within 9%, the unrelated probe is served 38x faster, and the
connection's work touches half the pool instead of all of it. A consumer that
needs the `lookup` figure on the async path is really asking for the
synchronous surface: on the same machine `rusqlite_prepared_statement_bench`
runs 100,000 of these lookups through `DbConnectionBlocking` in 37.7 ms against
473.6 ms for the async path, because it never crosses a thread at all (see
[Prepared-statement cache](prepared-statement-cache.md)).

Two permits would recover the pipelining, but the second admitted caller could
only sit on the mutex holding a blocking thread — exactly the starvation this
gate exists to stop, and the case the two-blocking-thread regression pins.

## How to run

```sh
cargo nextest run -p toolu-orm-connection --features rusqlite,sqlite-vec -E 'binary(rusqlite_async_backpressure_test)'
cargo run --release --example rusqlite_async_backpressure_bench -p toolu-orm-connection --features rusqlite
```

## Tests

| Lane | Binary | Test |
|---|---|---|
| rusqlite-only | rusqlite_async_backpressure_test | starvation::unrelated_blocking_work_is_served_while_a_statement_holds_the_connection |
| rusqlite-only | rusqlite_async_backpressure_test | cancellation::cancelling_before_admission_starts_no_sql |
| rusqlite-only | rusqlite_async_backpressure_test | cancellation::cancelling_after_admission_keeps_the_gate_closed |
| rusqlite-only | rusqlite_async_backpressure_test | serialization::concurrent_async_writers_all_land |
| rusqlite-only | rusqlite_async_backpressure_test | serialization::a_direct_blocking_caller_lands_while_async_callers_queue |
| rusqlite-only | rusqlite_async_backpressure_test | errors::a_decode_failure_is_still_a_row_mapping_error |
| rusqlite-only | rusqlite_async_backpressure_test | errors::a_panicking_decode_releases_the_permit |
