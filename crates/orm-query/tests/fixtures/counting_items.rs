//! The `item` table the first-row suites read, and decoders that count how many
//! rows a fetch actually maps.
//!
//! `#[from_row(with = "…")]` runs on the field's own decoded type inside each of
//! the derive's three driver decoders, so one derived struct counts rows in
//! every lane and no suite has to hand-write a per-driver `FromRow`.
//!
//! [`decoded`] reads a process-global counter. `cargo nextest` gives each test
//! its own process (and `cargo test` is banned by CLAUDE.md), so a
//! [`reset`]-then-assert pair is exact rather than merely indicative. Every
//! access is `SeqCst`: a counter read once per assertion has no hot path to
//! protect, and the strongest ordering keeps the exactness claim from resting
//! on which thread a driver happened to decode on.

use std::sync::atomic::{AtomicUsize, Ordering};

use toolu_orm_core::error::DbCoreError;
use toolu_orm_macros::FromRow;

/// How many rows every SQLite-flavoured suite seeds: large enough that
/// decoding all of them would be unmistakable in the counter.
pub const ROW_COUNT: i64 = 10_000;

pub const ITEM_DDL_SQLITE: &str = "CREATE TABLE item (id INTEGER PRIMARY KEY)";
pub const ITEM_DDL_POSTGRES: &str = "CREATE TABLE item (id BIGINT PRIMARY KEY)";

/// `INSERT` seeding `item` with ids `1..=n`, SQLite flavour.
pub fn seed_sql_sqlite(n: i64) -> String {
  format!(
    "WITH RECURSIVE seq(x) AS (VALUES(1) UNION ALL SELECT x + 1 FROM seq WHERE x < {n}) \
     INSERT INTO item (id) SELECT x FROM seq"
  )
}

/// `INSERT` seeding `item` with ids `1..=n`, Postgres flavour.
pub fn seed_sql_postgres(n: i64) -> String {
  format!("INSERT INTO item (id) SELECT generate_series(1, {n})")
}

static DECODED: AtomicUsize = AtomicUsize::new(0);

/// Zeroes the decode counter. Call it immediately before the fetch under test.
pub fn reset() {
  DECODED.store(0, Ordering::SeqCst);
}

/// Rows decoded since the last [`reset`].
pub fn decoded() -> usize {
  DECODED.load(Ordering::SeqCst)
}

/// Counts the decode and accepts any row of a seeded table, whose ids run
/// `1..=ROW_COUNT`; anything else means the fixture, not the fetch, is wrong.
fn count_id(id: i64) -> Result<i64, DbCoreError> {
  DECODED.fetch_add(1, Ordering::SeqCst);
  if id >= 1 {
    Ok(id)
  } else {
    Err(DbCoreError::RowMapping(format!(
      "item ids start at 1, got {id}"
    )))
  }
}

/// Counts the decode and accepts only `id == 1`, so any attempt to decode a
/// later row of an ascending scan surfaces as a `RowMapping` error.
fn only_first_id(id: i64) -> Result<i64, DbCoreError> {
  DECODED.fetch_add(1, Ordering::SeqCst);
  if id == 1 {
    Ok(id)
  } else {
    Err(DbCoreError::RowMapping(format!(
      "row {id} must not be decoded"
    )))
  }
}

/// One `item` row, counting every decode.
#[derive(FromRow, Debug, PartialEq, Eq)]
pub struct CountedId {
  #[from_row(with = "count_id")]
  pub id: i64,
}

/// One `item` row that refuses to decode anything but the first id.
#[derive(FromRow, Debug, PartialEq, Eq)]
pub struct FirstIdOnly {
  #[from_row(with = "only_first_id")]
  pub id: i64,
}
