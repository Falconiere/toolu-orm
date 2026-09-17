//! `.offset(n)` without a paired `.limit(...)` executes on a real in-memory
//! libsql database (libsql-only lane, async). The async twin of
//! `rusqlite_offset_without_limit_test`; see issue #92.

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
async fn seeded() -> Result<libsql::Connection, Box<dyn std::error::Error>> {
  let db = libsql::Builder::new_local(":memory:").build().await?;
  let conn = db.connect()?;
  conn.execute(ITEM_DDL_SQLITE, ()).await?;
  let inserted = conn.execute(&seed_sql_sqlite(ROWS), ()).await?;
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

#[tokio::test]
async fn offset_zero_returns_every_row() -> TestResult {
  let conn = seeded().await?;

  let rows: Vec<CountedId> = items()
    .order_by(ID.asc())
    .offset(0)
    .fetch_all(&conn)
    .await?;

  assert_eq!(ids(rows), (1..=ROWS).collect::<Vec<_>>());
  Ok(())
}

#[tokio::test]
async fn positive_offset_skips_the_leading_rows() -> TestResult {
  let conn = seeded().await?;

  let rows: Vec<CountedId> = items()
    .order_by(ID.asc())
    .offset(3)
    .fetch_all(&conn)
    .await?;

  assert_eq!(ids(rows), (4..=ROWS).collect::<Vec<_>>());
  Ok(())
}

/// An offset equal to the row count is a valid boundary: zero rows, not an
/// error.
#[tokio::test]
async fn offset_equal_to_row_count_returns_empty() -> TestResult {
  let conn = seeded().await?;

  let rows: Vec<CountedId> = items()
    .order_by(ID.asc())
    .offset(ROWS)
    .fetch_all(&conn)
    .await?;

  assert!(ids(rows).is_empty());
  Ok(())
}

/// An offset far beyond the table size must still execute and return no
/// rows, rather than the pre-fix syntax error.
#[tokio::test]
async fn offset_beyond_row_count_returns_empty() -> TestResult {
  let conn = seeded().await?;

  let rows: Vec<CountedId> = items()
    .order_by(ID.asc())
    .offset(1_000)
    .fetch_all(&conn)
    .await?;

  assert!(ids(rows).is_empty());
  Ok(())
}

/// A WHERE filter ahead of offset-only pagination proves parameter numbering
/// stays correct end to end, not just in the SQL-only render test.
#[tokio::test]
async fn where_filter_then_offset_only_selects_the_right_rows() -> TestResult {
  let conn = seeded().await?;

  let rows: Vec<CountedId> = items()
    .filter(ID.gt(2))
    .order_by(ID.asc())
    .offset(1)
    .fetch_all(&conn)
    .await?;

  assert_eq!(ids(rows), (4..=ROWS).collect::<Vec<_>>());
  Ok(())
}

#[tokio::test]
async fn explicit_limit_and_offset_still_page_correctly() -> TestResult {
  let conn = seeded().await?;

  let rows: Vec<CountedId> = items()
    .order_by(ID.asc())
    .limit(3)
    .offset(2)
    .fetch_all(&conn)
    .await?;

  assert_eq!(ids(rows), vec![3, 4, 5]);
  Ok(())
}
