//! The copy, its conflict modes, and what they leave behind.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::expr::Scalar;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;

use super::db;
use super::schema::{
  FileRow, TestResult, BLOB_BYTES, BLOB_OID, INDEXED_AT, PATH, RANK, REPO, SOURCE_COLUMNS,
};

fn target() -> TableRef {
  TableRef::new("file_index").in_database("main")
}

/// `legacy_index` of the same database, with `rank` projected as a literal.
fn older(rank: i64) -> SelectBuilder {
  SelectBuilder::from_table(TableRef::new("legacy_index").in_database("main"))
    .columns_raw(SOURCE_COLUMNS)
    .column_scalar(Scalar::bind(rank), "rank")
}

fn copy_with(builder: InsertBuilder, rank: i64) -> InsertBuilder {
  builder.select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK], older(rank))
}

/// Every landed row, ordered by key.
async fn landed(conn: &libsql::Connection) -> Result<Vec<FileRow>, toolu_orm_query::QueryError> {
  SelectBuilder::new("file_index")
    .columns_raw(&["repo", "path", "blob_oid", "indexed_at", "rank"])
    .order_by(REPO.asc())
    .order_by(PATH.asc())
    .fetch_all(conn)
    .await
}

/// The landed row with this path, or an error naming what was there instead.
fn row_at<'a>(rows: &'a [FileRow], path: &str) -> Result<&'a FileRow, Box<dyn std::error::Error>> {
  rows
    .iter()
    .find(|row| row.path == path)
    .ok_or_else(|| format!("no row at {path}; landed: {rows:?}").into())
}

/// Pre-seed the target with the key the source also has.
async fn seed_conflicting_row(conn: &libsql::Connection) -> TestResult {
  InsertBuilder::new("file_index")
    .set(&REPO, "r1")
    .set(&PATH, "src/a.rs")
    .set(&INDEXED_AT, 999_i64)
    .set(&RANK, 42_i64)
    .execute(conn)
    .await?;
  Ok(())
}

#[tokio::test]
async fn one_statement_copies_every_row_with_its_nulls_and_blobs() -> TestResult {
  let conn = db::setup_db().await?;

  let affected = copy_with(InsertBuilder::into_table(target()), 7)
    .execute(&conn)
    .await?;
  assert_eq!(affected, 3);

  let rows = landed(&conn).await?;
  assert_eq!(rows.len(), 3);
  assert_eq!(
    row_at(&rows, "src/a.rs")?.blob_oid.as_deref(),
    Some(BLOB_BYTES)
  );
  let nulled = row_at(&rows, "src/b.rs")?;
  assert_eq!(nulled.blob_oid, None, "a NULL blob stays NULL");
  assert_eq!(nulled.indexed_at, None, "a NULL integer stays NULL");
  assert_eq!(
    row_at(&rows, "src/c.rs")?.blob_oid.as_deref(),
    Some(&[][..]),
    "an empty blob is a value, distinct from NULL"
  );
  assert!(
    rows.iter().all(|r| r.rank == 7),
    "the projected default, not the table DEFAULT"
  );
  Ok(())
}

#[tokio::test]
async fn or_ignore_keeps_the_stored_row_and_copies_the_rest() -> TestResult {
  let conn = db::setup_db().await?;
  seed_conflicting_row(&conn).await?;

  let affected = copy_with(InsertBuilder::into_table(target()).or_ignore(), 7)
    .execute(&conn)
    .await?;
  assert_eq!(affected, 2, "one key was already present");

  let rows = landed(&conn).await?;
  let kept = row_at(&rows, "src/a.rs")?;
  assert_eq!(kept.indexed_at, Some(999), "the stored row survived");
  assert_eq!(kept.rank, 42);
  assert_eq!(rows.len(), 3);
  Ok(())
}

#[tokio::test]
async fn an_explicit_conflict_clause_parses_and_updates_the_stored_row() -> TestResult {
  let conn = db::setup_db().await?;
  seed_conflicting_row(&conn).await?;

  // Without the SQLite derived-table guard libsql reads this `ON` as a join's.
  let affected = copy_with(InsertBuilder::into_table(target()), 7)
    .on_conflict(
      OnConflict::column(&REPO)
        .and_column(&PATH)
        .set_excluded(&INDEXED_AT)
        .set(&RANK, 5_i64),
    )
    .execute(&conn)
    .await?;
  assert_eq!(affected, 3, "two inserts and one update");

  let rows = landed(&conn).await?;
  let updated = row_at(&rows, "src/a.rs")?;
  assert_eq!(
    updated.indexed_at,
    Some(100),
    "the proposed value replaced 999"
  );
  assert_eq!(updated.rank, 5);
  assert_eq!(rows.len(), 3);
  Ok(())
}

#[tokio::test]
async fn the_unqualified_target_lands_the_same_rows_as_the_qualified_one() -> TestResult {
  let conn = db::setup_db().await?;
  copy_with(InsertBuilder::new("file_index"), 7)
    .execute(&conn)
    .await?;

  let rows = landed(&conn).await?;
  assert_eq!(
    rows.len(),
    3,
    "\"main\" is the default schema, so both name it"
  );
  Ok(())
}
