//! Release benchmark for issue #90 (rusqlite async admission gate).
//!
//! Sixteen concurrent tasks share one `RusqliteConnection` on a runtime whose
//! blocking pool holds four threads. `ungated` is the pre-change async path
//! (`spawn_blocking` around `DbConnectionBlocking`, so every contending caller
//! occupies a blocking thread while it waits for the connection's mutex);
//! `gated` is the async `DbConnection` methods, which take one shared permit
//! first. Two workloads: `lookup`, a one-row indexed read, isolates the
//! per-call handoff the gate serializes; `scan`, a 1,000-row read, is the case
//! where the connection itself is the bottleneck. Each run reports elapsed
//! time, calls per second, how many distinct blocking threads the connection's
//! work touched, and how long an unrelated `spawn_blocking` waited for a thread
//! while the workload ran. Both variants must decode the same checksum.
//!
//! Run with:
//!   cargo run --release --example rusqlite_async_backpressure_bench \
//!     -p toolu-orm-connection --features rusqlite

use std::cell::Cell;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_connection::{DbConnection, DbConnectionBlocking, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

const ROWS: i64 = 1_000;
const TASKS: i64 = 16;
const POOL: usize = 4;
const PROBE_BUDGET: Duration = Duration::from_secs(10);
const DDL: &str = "CREATE TABLE items(id INTEGER PRIMARY KEY, name TEXT NOT NULL); \
  WITH RECURSIVE seq(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM seq WHERE n<1000) \
  INSERT INTO items SELECT n, 'item-' || n FROM seq;";

/// A one-row indexed read, and a full 1,000-row read. Both decode every row
/// they return, so the sum of the decoded ids is fixed and the two variants of
/// a workload must agree on it.
#[derive(Clone, Copy)]
struct Workload {
  name: &'static str,
  sql: &'static str,
  parameterized: bool,
  calls_per_task: i64,
  checksum: i64,
}

/// `calls_per_task` is a multiple of `ROWS`, so each task walks every id the
/// same number of times whatever its starting offset.
const LOOKUP: Workload = Workload {
  name: "lookup",
  sql: "SELECT id, name FROM items WHERE id = ?1",
  parameterized: true,
  calls_per_task: 5_000,
  checksum: TASKS * (5_000 / ROWS) * (ROWS * (ROWS + 1) / 2),
};

const SCAN: Workload = Workload {
  name: "scan",
  sql: "SELECT id, name FROM items",
  parameterized: false,
  calls_per_task: 100,
  checksum: TASKS * 100 * (ROWS * (ROWS + 1) / 2),
};

/// Distinct blocking threads that ran connection work in the current run. Every
/// run builds its own runtime, so its pool is a fresh set of threads with fresh
/// thread-locals and the count starts clean.
static BLOCKING_THREADS: AtomicUsize = AtomicUsize::new(0);

thread_local! {
  static COUNTED: Cell<bool> = const { Cell::new(false) };
}

/// One thread-local read per decode, so the measurement adds no cross-thread
/// contention to the path being measured.
fn note_blocking_thread() {
  COUNTED.with(|counted| {
    if !counted.get() {
      counted.set(true);
      BLOCKING_THREADS.fetch_add(1, Ordering::Relaxed);
    }
  });
}

struct Item {
  id: i64,
  name: String,
}

impl FromRow for Item {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "name"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    note_blocking_thread();
    let id: i64 = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    let name: String = row
      .get(1)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { id, name })
  }
}

fn setup() -> Result<Arc<RusqliteConnection>, DbError> {
  let raw =
    rusqlite::Connection::open_in_memory().map_err(|e| DbError::Connection(e.to_string()))?;
  let conn = RusqliteConnection::from_connection(raw);
  DbConnectionBlocking::execute_batch(&conn, DDL)?;
  Ok(Arc::new(conn))
}

fn runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
  tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)
    .max_blocking_threads(POOL)
    .enable_all()
    .build()
}

fn joined(error: &tokio::task::JoinError) -> DbError {
  DbError::Connection(format!("a benchmark task did not complete: {error}"))
}

fn total(rows: Vec<Item>) -> i64 {
  rows.into_iter().fold(0, |sum, item| {
    black_box(item.name);
    sum + item.id
  })
}

/// One task's share of a workload, on either path. `ungated` reproduces the
/// pre-change shape: straight into the blocking pool, then wait for the
/// connection's mutex there.
async fn run_task(
  conn: Arc<RusqliteConnection>,
  task: i64,
  workload: Workload,
  gated: bool,
) -> Result<i64, DbError> {
  let mut sum = 0_i64;
  for call in 0..workload.calls_per_task {
    let params = if workload.parameterized {
      vec![Value::Integer((task + call) % ROWS + 1)]
    } else {
      vec![]
    };
    sum += if gated {
      total(DbConnection::query_map(&*conn, workload.sql, params).await?)
    } else {
      let target = Arc::clone(&conn);
      let rows = tokio::task::spawn_blocking(move || {
        DbConnectionBlocking::query_map(&*target, workload.sql, params)
      })
      .await
      .map_err(|e| joined(&e))??;
      total(rows)
    };
  }
  Ok(sum)
}

/// How long an unrelated `spawn_blocking` waits for a thread while the workload
/// runs. `None` means it was still starved when the budget ran out.
async fn probe_unrelated_work() -> Option<Duration> {
  let started = Instant::now();
  let probe = tokio::task::spawn_blocking(|| black_box(7_i32));
  match tokio::time::timeout(PROBE_BUDGET, probe).await {
    Ok(Ok(_)) => Some(started.elapsed()),
    Ok(Err(_)) | Err(_) => None,
  }
}

fn probe_label(probe: Option<Duration>) -> String {
  match probe {
    Some(waited) => format!("{waited:?}"),
    None => format!("starved >{PROBE_BUDGET:?}"),
  }
}

/// Runs one variant of one workload on its own runtime: spawn every task, probe
/// the pool while they contend, then collect the checksum.
fn measure(
  workload: Workload,
  gated: bool,
  conn: &Arc<RusqliteConnection>,
) -> Result<i64, Box<dyn std::error::Error>> {
  BLOCKING_THREADS.store(0, Ordering::Relaxed);
  let runtime = runtime()?;
  let started = Instant::now();
  let (checksum, probe) = runtime.block_on(async {
    let mut tasks = Vec::new();
    for task in 0..TASKS {
      let target = Arc::clone(conn);
      tasks.push(tokio::spawn(run_task(target, task, workload, gated)));
    }
    tokio::time::sleep(Duration::from_millis(25)).await;
    let probe = probe_unrelated_work().await;
    let mut sum = 0_i64;
    for task in tasks {
      sum += task.await.map_err(|e| joined(&e))??;
    }
    Ok::<(i64, Option<Duration>), DbError>((sum, probe))
  })?;
  let elapsed = started.elapsed();
  let calls = u128::try_from(TASKS * workload.calls_per_task)?;
  println!(
    "{:<7} {:<8} elapsed={elapsed:?} calls/s={} blocking_threads={} unrelated_probe={}",
    workload.name,
    if gated { "gated" } else { "ungated" },
    calls * 1_000 / elapsed.as_millis().max(1),
    BLOCKING_THREADS.load(Ordering::Relaxed),
    probe_label(probe),
  );
  Ok(checksum)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let conn = setup()?;
  for workload in [LOOKUP, SCAN] {
    let expected = workload.checksum;
    let ungated = measure(workload, false, &conn)?;
    let gated = measure(workload, true, &conn)?;
    if ungated != expected || gated != expected {
      return Err(
        format!(
          "{} checksum mismatch: ungated={ungated}, gated={gated}, expected={expected}",
          workload.name
        )
        .into(),
      );
    }
    println!("{} variants agree on checksum={expected}", workload.name);
  }
  println!("sqlite_version={}", rusqlite::version());
  Ok(())
}
