//! `.offset(n)` without a paired `.limit(...)` executes on a real in-memory
//! rusqlite database (rusqlite-only lane, synchronous). Before this fix the
//! rendered SQL was a bare `OFFSET`, which SQLite's `sqlite3_prepare` rejects
//! as a syntax error at `fetch_all` time. See issue #92.

#[path = "fixtures/counting_items.rs"]
pub mod items;

use items::{seed_sql_sqlite, CountedId, ITEM_DDL_SQLITE};
use toolu_orm_core::column::Integer;
use toolu_orm_core::query_column::{Column, NumericOps};
use toolu_orm_query::select::SelectBuilder;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ID: Column<Integer> = Column::new("item", "id");
const ROWS: i64 = 10;

/// An in-memory database whose `item` table holds ids `1..=ROWS`.
fn seeded() -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute(ITEM_DDL_SQLITE, ())?;
  let inserted = conn.execute(&seed_sql_sqlite(ROWS), ())?;
  assert_eq!(
    i64::try_from(inserted)?,
    ROWS,
    "seed inserted the wrong count"
  );
  Ok(conn)
}

fn items() -> SelectBuilder {
  SelectBuilder::new("item").columns_raw(&["id"])
}

fn ids(rows: Vec<CountedId>) -> Vec<i64> {
  rows.into_iter().map(|row| row.id).collect()
}

#[test]
fn offset_zero_returns_every_row() -> TestResult {
  let conn = seeded()?;

  let rows: Vec<CountedId> = items().order_by(ID.asc()).offset(0).fetch_all(&conn)?;

  assert_eq!(ids(rows), (1..=ROWS).collect::<Vec<_>>());
  Ok(())
}

#[test]
fn positive_offset_skips_the_leading_rows() -> TestResult {
  let conn = seeded()?;

  let rows: Vec<CountedId> = items().order_by(ID.asc()).offset(3).fetch_all(&conn)?;

  assert_eq!(ids(rows), (4..=ROWS).collect::<Vec<_>>());
  Ok(())
}

/// An offset equal to the row count is a valid boundary: zero rows, not an
/// error.
#[test]
fn offset_equal_to_row_count_returns_empty() -> TestResult {
  let conn = seeded()?;

  let rows: Vec<CountedId> = items().order_by(ID.asc()).offset(ROWS).fetch_all(&conn)?;

  assert!(ids(rows).is_empty());
  Ok(())
}

/// An offset far beyond the table size must still execute and return no
/// rows, rather than the pre-fix syntax error.
#[test]
fn offset_beyond_row_count_returns_empty() -> TestResult {
  let conn = seeded()?;

  let rows: Vec<CountedId> = items().order_by(ID.asc()).offset(1_000).fetch_all(&conn)?;

  assert!(ids(rows).is_empty());
  Ok(())
}

/// A WHERE filter ahead of offset-only pagination, plus an explicit limit
/// alongside offset elsewhere in the same table — proves parameter numbering
/// stays correct end to end, not just in the SQL-only render test.
#[test]
fn where_filter_then_offset_only_selects_the_right_rows() -> TestResult {
  let conn = seeded()?;

  let rows: Vec<CountedId> = items()
    .filter(ID.gt(2))
    .order_by(ID.asc())
    .offset(1)
    .fetch_all(&conn)?;

  assert_eq!(ids(rows), (4..=ROWS).collect::<Vec<_>>());
  Ok(())
}

#[test]
fn explicit_limit_and_offset_still_page_correctly() -> TestResult {
  let conn = seeded()?;

  let rows: Vec<CountedId> = items()
    .order_by(ID.asc())
    .limit(3)
    .offset(2)
    .fetch_all(&conn)?;

  assert_eq!(ids(rows), vec![3, 4, 5]);
  Ok(())
}
