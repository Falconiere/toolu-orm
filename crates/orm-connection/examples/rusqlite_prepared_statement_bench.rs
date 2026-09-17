//! Release benchmark for issue #88 (rusqlite prepared-statement reuse).
//!
//! On the same 1,000-row in-memory table, with 100,000 parameterized indexed
//! lookups per variant, this compares:
//!
//! - `uncached` -- `Connection::prepare` on every call, the pre-change shape.
//! - `cached` -- `Connection::prepare_cached` on every call, mirroring
//!   `Executor::query_map` for `rusqlite::Connection` after this change
//!   (`crates/orm-query/src/executor/rusqlite_impl.rs`).
//! - `prepare_only` -- `prepare_cached` alone, no bind/execute/decode, to
//!   isolate preparation cost from row decoding.
//! - `blocking` -- `RusqliteConnection` through `DbConnectionBlocking`
//!   (wrapper + `std::sync::Mutex` lock), this crate's own post-change code.
//! - `async` -- the same `RusqliteConnection` through `DbConnection`, which
//!   hands off to `blocking`'s code inside `tokio::task::spawn_blocking`
//!   (untouched by this change; isolates thread-handoff cost).
//!
//! Every variant decodes the same rows; the run asserts equal checksums
//! across `uncached`, `cached`, `blocking`, and `async` before printing.
//!
//! Run with:
//!   cargo run --release --example rusqlite_prepared_statement_bench \
//!     -p toolu-orm-connection --features rusqlite

use std::hint::black_box;
use std::time::{Duration, Instant};

use toolu_orm_connection::rusqlite_impl::RusqliteConnection;
use toolu_orm_connection::{DbConnection, DbConnectionBlocking, DbError};
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::row::FromRow;
use toolu_orm_core::value::Value;

const ROWS: i64 = 1_000;
const LOOKUPS: i64 = 100_000;
const SELECT_SQL: &str = "SELECT id, name FROM items WHERE id = ?1";
const DDL: &str = "CREATE TABLE items(id INTEGER PRIMARY KEY, name TEXT NOT NULL); \
  WITH RECURSIVE seq(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM seq WHERE n<1000) \
  INSERT INTO items SELECT n, 'item-' || n FROM seq;";

/// The checksum every variant must agree on: each id in `1..=ROWS` is looked
/// up exactly `LOOKUPS / ROWS` times, and a variant sums the decoded ids.
fn expected_checksum() -> i64 {
  (LOOKUPS / ROWS) * (ROWS * (ROWS + 1) / 2)
}

struct Item {
  id: i64,
  name: String,
}

impl FromRow for Item {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["id", "name"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    let id: i64 = row
      .get(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    let name: String = row
      .get(1)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self { id, name })
  }
}

fn setup_bare() -> Result<rusqlite::Connection, rusqlite::Error> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute_batch(DDL)?;
  Ok(conn)
}

fn setup_wrapped() -> Result<RusqliteConnection, DbError> {
  let conn =
    rusqlite::Connection::open_in_memory().map_err(|e| DbError::Connection(e.to_string()))?;
  let rc = RusqliteConnection::from_connection(conn);
  DbConnectionBlocking::execute_batch(&rc, DDL)?;
  Ok(rc)
}

fn run_uncached(conn: &rusqlite::Connection) -> Result<i64, Box<dyn std::error::Error>> {
  let mut total = 0_i64;
  for i in 0..LOOKUPS {
    let id = i % ROWS + 1;
    let mut stmt = conn.prepare(SELECT_SQL)?;
    let mut rows = stmt.query([id])?;
    while let Some(row) = rows.next()? {
      let item = Item::from_row(row)?;
      total += item.id;
      black_box(&item.name);
    }
  }
  Ok(total)
}

fn run_cached(conn: &rusqlite::Connection) -> Result<i64, Box<dyn std::error::Error>> {
  let mut total = 0_i64;
  for i in 0..LOOKUPS {
    let id = i % ROWS + 1;
    let mut stmt = conn.prepare_cached(SELECT_SQL)?;
    let mut rows = stmt.query([id])?;
    while let Some(row) = rows.next()? {
      let item = Item::from_row(row)?;
      total += item.id;
      black_box(&item.name);
    }
  }
  Ok(total)
}

fn run_prepare_only(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
  for _ in 0..LOOKUPS {
    let stmt = conn.prepare_cached(SELECT_SQL)?;
    black_box(&stmt);
  }
  Ok(())
}

fn run_blocking(conn: &RusqliteConnection) -> Result<i64, DbError> {
  let mut total = 0_i64;
  for i in 0..LOOKUPS {
    let id = i % ROWS + 1;
    let rows: Vec<Item> =
      DbConnectionBlocking::query_map(conn, SELECT_SQL, vec![Value::Integer(id)])?;
    for item in rows {
      total += item.id;
      black_box(item.name);
    }
  }
  Ok(total)
}

async fn run_async(conn: &RusqliteConnection) -> Result<i64, DbError> {
  let mut total = 0_i64;
  for i in 0..LOOKUPS {
    let id = i % ROWS + 1;
    let rows: Vec<Item> =
      DbConnection::query_map(conn, SELECT_SQL, vec![Value::Integer(id)]).await?;
    for item in rows {
      total += item.id;
      black_box(item.name);
    }
  }
  Ok(total)
}

fn report(label: &str, elapsed: Duration, checksum: Option<i64>) {
  match checksum {
    Some(sum) => println!("{label:<13} lookups={LOOKUPS} elapsed={elapsed:?} checksum={sum}",),
    None => println!("{label:<13} lookups={LOOKUPS} elapsed={elapsed:?}"),
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let expected = expected_checksum();

  let bare = setup_bare()?;
  let start = Instant::now();
  let uncached_sum = run_uncached(&bare)?;
  report("uncached", start.elapsed(), Some(uncached_sum));

  let start = Instant::now();
  let cached_sum = run_cached(&bare)?;
  report("cached", start.elapsed(), Some(cached_sum));

  let start = Instant::now();
  run_prepare_only(&bare)?;
  report("prepare_only", start.elapsed(), None);

  let wrapped = setup_wrapped()?;
  let start = Instant::now();
  let blocking_sum = run_blocking(&wrapped)?;
  report("blocking", start.elapsed(), Some(blocking_sum));

  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()?;
  let start = Instant::now();
  let async_sum = runtime.block_on(run_async(&wrapped))?;
  report("async", start.elapsed(), Some(async_sum));

  assert_eq!(uncached_sum, expected, "uncached checksum mismatch");
  assert_eq!(cached_sum, expected, "cached checksum mismatch");
  assert_eq!(blocking_sum, expected, "blocking checksum mismatch");
  assert_eq!(async_sum, expected, "async checksum mismatch");
  println!("all variants agree on checksum={expected}");

  println!("sqlite_version={}", rusqlite::version());
  Ok(())
}
