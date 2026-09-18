//! Conflict handling on the copy: `INSERT OR IGNORE` really reaching the
//! engine, the error when nothing guards the key, and the explicit
//! `ON CONFLICT` clause SQLite would otherwise refuse to parse.

use toolu_orm_core::alias::TableRef;
use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_core::row::FromRow;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

use super::db::{target_rows, TestResult, BLOB_OID, INDEXED_AT, PATH, RANK, REPO};
use super::support::{Attached, OLD};

/// The older store's `indexed_files`, projecting `rank` as `default_rank`.
fn older(default_rank: i64) -> SelectBuilder {
  SelectBuilder::from_table(TableRef::new("indexed_files").in_database(OLD))
    .columns_raw(&["repo", "path", "blob_oid", "indexed_at"])
    .column_scalar(Scalar::bind(default_rank), "rank")
}

fn target() -> TableRef {
  TableRef::new("indexed_files").in_database("main")
}

/// Pre-seed the target with `("r1", "src/a.rs")`, the key the source also has.
fn seed_conflicting_row(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
  conn.execute(
    "INSERT INTO main.indexed_files (repo, path, blob_oid, indexed_at, rank) \
     VALUES ('r1', 'src/a.rs', NULL, 999, 42)",
    [],
  )?;
  Ok(())
}

#[test]
fn or_ignore_skips_the_conflicting_row_and_keeps_the_stored_one() -> TestResult {
  let fixture = Attached::open("or-ignore")?;
  let attached = fixture.attach()?;
  seed_conflicting_row(&fixture.conn)?;

  let affected = InsertBuilder::into_table(target())
    .or_ignore()
    .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK], older(7))
    .execute(&fixture.conn)?;
  assert_eq!(affected, 2, "one of the three keys was already present");
  drop(attached);

  let rows = target_rows(&fixture.conn)?;
  let Some(kept) = rows.iter().find(|r| r.path == "src/a.rs") else {
    return Err("the pre-seeded row disappeared".into());
  };
  assert_eq!(
    kept.indexed_at,
    Some(999),
    "the stored row was not overwritten"
  );
  assert_eq!(kept.rank, 42);
  assert_eq!(rows.len(), 3);
  Ok(())
}

#[test]
fn without_a_conflict_mode_the_same_copy_reports_the_uniqueness_violation() -> TestResult {
  let fixture = Attached::open("unguarded")?;
  let attached = fixture.attach()?;
  seed_conflicting_row(&fixture.conn)?;

  let outcome = InsertBuilder::into_table(target())
    .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK], older(7))
    .execute(&fixture.conn);
  drop(attached);

  let Err(QueryError::Driver(message)) = outcome else {
    return Err(format!("expected a driver error, got {outcome:?}").into());
  };
  let text = message.to_string().to_uppercase();
  assert!(
    text.contains("UNIQUE"),
    "SQLite should name the constraint: {message}"
  );
  Ok(())
}

#[test]
fn an_explicit_conflict_clause_parses_and_updates_the_stored_row() -> TestResult {
  let fixture = Attached::open("upsert")?;
  let attached = fixture.attach()?;
  seed_conflicting_row(&fixture.conn)?;

  // Without the derived-table guard SQLite reads this statement's `ON` as a
  // join's and refuses it with `Parse error: near "DO"`. Reaching the engine at
  // all is therefore half of what this asserts.
  let affected = InsertBuilder::into_table(target())
    .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK], older(7))
    .on_conflict(
      OnConflict::column(&REPO)
        .and_column(&PATH)
        .set_excluded(&INDEXED_AT)
        .set(&RANK, 5_i64),
    )
    .execute(&fixture.conn)?;
  assert_eq!(affected, 3, "two inserts and one update");
  drop(attached);

  let rows = target_rows(&fixture.conn)?;
  let Some(updated) = rows.iter().find(|r| r.path == "src/a.rs") else {
    return Err("the conflicting row disappeared".into());
  };
  assert_eq!(
    updated.indexed_at,
    Some(100),
    "the proposed row's value replaced the stored 999"
  );
  assert_eq!(updated.rank, 5);
  assert_eq!(rows.len(), 3);
  Ok(())
}

/// A `RETURNING`-projected row. The column is really decoded — a decode
/// failure would be a different error than the `NotFound` under test — and
/// then discarded, because only *whether* a row came back matters here.
#[derive(Debug)]
struct ReturnedRepo;

impl FromRow for ReturnedRepo {
  const REQUIRED_COLUMNS: &'static [&'static str] = &["repo"];

  fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, DbCoreError> {
    row
      .get::<_, String>(0)
      .map_err(|e| DbCoreError::RowMapping(e.to_string()))?;
    Ok(Self)
  }
}

#[test]
fn a_copy_that_ignores_every_row_reports_not_found_under_the_bare_table_name() -> TestResult {
  let fixture = Attached::open("not-found")?;
  let attached = fixture.attach()?;
  // Every source key already exists, so OR IGNORE inserts nothing and
  // RETURNING projects no row at all.
  fixture.conn.execute(
    "INSERT INTO main.indexed_files (repo, path, rank) \
     SELECT repo, path, 0 FROM old.indexed_files",
    [],
  )?;

  let outcome = InsertBuilder::into_table(target())
    .or_ignore()
    .select(&[&REPO, &PATH, &BLOB_OID, &INDEXED_AT, &RANK], older(7))
    .returning(&REPO)
    .fetch_one::<ReturnedRepo>(&fixture.conn);
  drop(attached);

  let Err(QueryError::NotFound { table }) = outcome else {
    return Err(format!("expected NotFound, got {outcome:?}").into());
  };
  assert_eq!(
    table, "indexed_files",
    "the error names the bare table, not the qualified \"main\".\"indexed_files\""
  );
  Ok(())
}
