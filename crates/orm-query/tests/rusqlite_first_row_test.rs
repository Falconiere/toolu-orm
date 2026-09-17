//! `fetch_one` / `fetch_optional` decode at most one row (rusqlite-only lane,
//! synchronous), against an in-memory database holding 10,000 matching rows.
//!
//! Issue #87: both methods used to run the unbounded query, decode every row
//! and drop all but the first, so a large match cost thousands of decodes and a
//! later row that could not decode failed an otherwise valid first row.

#[path = "fixtures/counting_items.rs"]
pub mod items;

use items::{decoded, reset, seed_sql_sqlite, CountedId, FirstIdOnly, ITEM_DDL_SQLITE, ROW_COUNT};
use toolu_orm_core::column::Integer;
use toolu_orm_core::query_column::{Column, NumericOps};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ID: Column<Integer> = Column::new("item", "id");

/// An in-memory database whose `item` table holds ids `1..=rows`.
///
/// The seed's own row count is asserted, so every decode assertion below rests
/// on a table proven to hold `rows` rows rather than on an assumption.
fn seeded(rows: i64) -> Result<rusqlite::Connection, Box<dyn std::error::Error>> {
  let conn = rusqlite::Connection::open_in_memory()?;
  conn.execute(ITEM_DDL_SQLITE, ())?;
  let inserted = conn.execute(&seed_sql_sqlite(rows), ())?;
  assert_eq!(
    i64::try_from(inserted)?,
    rows,
    "seed inserted the wrong count"
  );
  reset();
  Ok(conn)
}

fn items() -> SelectBuilder {
  SelectBuilder::new("item").columns_raw(&["id"])
}

#[test]
fn fetch_one_decodes_one_row_for_a_large_match() -> TestResult {
  let conn = seeded(ROW_COUNT)?;

  let first: CountedId = items().order_by(ID.asc()).fetch_one(&conn)?;

  assert_eq!(first.id, 1);
  assert_eq!(decoded(), 1, "fetch_one must decode exactly one row");
  Ok(())
}

#[test]
fn fetch_optional_decodes_one_row_for_a_large_match() -> TestResult {
  let conn = seeded(ROW_COUNT)?;

  let first: Option<CountedId> = items().order_by(ID.asc()).fetch_optional(&conn)?;

  assert_eq!(first.map(|row| row.id), Some(1));
  assert_eq!(decoded(), 1, "fetch_optional must decode exactly one row");
  Ok(())
}

#[test]
fn first_row_survives_a_later_row_that_cannot_decode() -> TestResult {
  let conn = seeded(ROW_COUNT)?;

  let one: FirstIdOnly = items().order_by(ID.asc()).fetch_one(&conn)?;
  let optional: Option<FirstIdOnly> = items().order_by(ID.asc()).fetch_optional(&conn)?;

  assert_eq!(one.id, 1);
  assert_eq!(optional.map(|row| row.id), Some(1));
  Ok(())
}

#[test]
fn no_matching_row_is_not_found_and_none() -> TestResult {
  let conn = seeded(ROW_COUNT)?;

  let missing: Result<CountedId, QueryError> = items().filter(ID.gt(ROW_COUNT)).fetch_one(&conn);
  let optional: Option<CountedId> = items().filter(ID.gt(ROW_COUNT)).fetch_optional(&conn)?;

  match missing {
    Err(QueryError::NotFound { table }) => assert_eq!(table, "item"),
    other => return Err(format!("expected NotFound, got: {other:?}").into()),
  }
  assert!(optional.is_none());
  assert_eq!(decoded(), 0);
  Ok(())
}

#[test]
fn explicit_limit_zero_yields_no_row() -> TestResult {
  let conn = seeded(ROW_COUNT)?;

  let one: Result<CountedId, QueryError> = items().limit(0).fetch_one(&conn);
  let optional: Option<CountedId> = items().limit(0).fetch_optional(&conn)?;

  match one {
    Err(QueryError::NotFound { table }) => assert_eq!(table, "item"),
    other => return Err(format!("expected NotFound, got: {other:?}").into()),
  }
  assert!(optional.is_none());
  assert_eq!(decoded(), 0);
  Ok(())
}

#[test]
fn filters_order_offset_and_positive_limit_still_select_the_first_row() -> TestResult {
  let conn = seeded(ROW_COUNT)?;

  let skipped: CountedId = items()
    .filter(ID.lte(ROW_COUNT))
    .order_by(ID.desc())
    .offset(2)
    .fetch_one(&conn)?;
  assert_eq!(skipped.id, ROW_COUNT - 2);
  assert_eq!(decoded(), 1);

  reset();
  let paged: CountedId = items().order_by(ID.asc()).limit(5).fetch_one(&conn)?;
  assert_eq!(paged.id, 1);
  assert_eq!(decoded(), 1, "an explicit page of 5 still decodes one row");
  Ok(())
}

#[test]
fn decoded_rows_stay_one_as_cardinality_grows() -> TestResult {
  for rows in [1, 10, 1_000, ROW_COUNT] {
    let conn = seeded(rows)?;

    let first: CountedId = items().order_by(ID.asc()).fetch_one(&conn)?;

    assert_eq!(first.id, 1);
    assert_eq!(decoded(), 1, "decoded more than one row at {rows} rows");
  }
  Ok(())
}
