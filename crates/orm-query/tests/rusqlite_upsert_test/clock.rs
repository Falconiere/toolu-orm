//! Expression `VALUES`: a timestamp computed by the database, with both
//! `strftime` arguments bound as parameters.

use toolu_orm_core::error::DbCoreError;
use toolu_orm_core::expr::Scalar;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;

use super::db;
use super::schema::{
  CodeSymbol, GeneratedId, INDEXED_AT, ISO_FORMAT, PATH, REPO, SYMBOL_COLUMNS, SYMBOL_ID,
};
use super::support::TestResult;

fn database_clock() -> Result<Scalar, DbCoreError> {
  Scalar::func(
    "strftime",
    vec![Scalar::bind(ISO_FORMAT), Scalar::bind("now")],
  )
}

fn index_with_clock(path: &str) -> Result<InsertBuilder, DbCoreError> {
  Ok(
    InsertBuilder::new("code_symbols")
      .set(&REPO, "toolu-orm")
      .set(&PATH, path)
      .set_scalar(&INDEXED_AT, database_clock()?)
      .on_conflict(
        OnConflict::column(&REPO)
          .and_column(&PATH)
          .set_scalar(&INDEXED_AT, database_clock()?),
      )
      .returning(&SYMBOL_ID),
  )
}

fn only_symbol(conn: &rusqlite::Connection) -> Result<CodeSymbol, toolu_orm_query::QueryError> {
  SelectBuilder::new("code_symbols")
    .columns_raw(&SYMBOL_COLUMNS)
    .fetch_one(conn)
}

#[test]
fn a_database_clock_expression_writes_an_iso_timestamp() -> TestResult {
  let conn = db::setup_db()?;

  let written: GeneratedId = index_with_clock("src/lib.rs")?.fetch_one(&conn)?;
  assert_eq!(written.id, 1);

  let stamp = only_symbol(&conn)?.indexed_at;
  assert_eq!(
    stamp.len(),
    24,
    "strftime('{ISO_FORMAT}', 'now') is YYYY-MM-DDTHH:MM:SS.sssZ, got {stamp}"
  );
  assert!(stamp.ends_with('Z'), "got {stamp}");
  assert_eq!(stamp.get(4..5), Some("-"), "got {stamp}");
  assert_eq!(stamp.get(10..11), Some("T"), "got {stamp}");
  Ok(())
}

#[test]
fn the_conflict_branch_recomputes_the_clock_from_its_own_binds() -> TestResult {
  let conn = db::setup_db()?;

  InsertBuilder::new("code_symbols")
    .set(&REPO, "toolu-orm")
    .set(&PATH, "src/lib.rs")
    .set(&INDEXED_AT, "1970-01-01T00:00:00.000Z")
    .execute(&conn)?;

  let again: GeneratedId = index_with_clock("src/lib.rs")?.fetch_one(&conn)?;
  assert_eq!(again.id, 1, "the stored row was updated in place");

  let stamp = only_symbol(&conn)?.indexed_at;
  assert!(
    stamp.as_str() > "1970-01-01T00:00:00.000Z",
    "the DO UPDATE branch must evaluate its own strftime, got {stamp}"
  );
  assert_eq!(stamp.len(), 24, "got {stamp}");
  Ok(())
}
