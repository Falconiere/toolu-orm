//! `RETURNING` over a generated id: what an upsert hands back, and how that
//! differs from `INSERT OR REPLACE` allocating a fresh row.

use toolu_orm_core::query_column::CommonOps;
use toolu_orm_query::insert::{InsertBuilder, OnConflict};
use toolu_orm_query::select::SelectBuilder;
use toolu_orm_query::QueryError;

use super::db;
use super::schema::{
  CodeSymbol, GeneratedId, BODY, INDEXED_AT, MEMORY_ID, PATH, REPO, SYMBOL_COLUMNS, SYMBOL_ID,
};
use super::support::{seed_memory, TestResult};

fn index_symbol(repo: &str, path: &str, stamp: &str) -> InsertBuilder {
  InsertBuilder::new("code_symbols")
    .set(&REPO, repo)
    .set(&PATH, path)
    .set(&INDEXED_AT, stamp)
    .on_conflict(
      OnConflict::column(&REPO)
        .and_column(&PATH)
        .set_excluded(&INDEXED_AT),
    )
}

#[test]
fn returning_hands_back_the_generated_id_and_keeps_it_across_a_conflict() -> TestResult {
  let conn = db::setup_db()?;

  let first: GeneratedId = index_symbol("toolu-orm", "src/lib.rs", "t1")
    .returning(&SYMBOL_ID)
    .fetch_one(&conn)?;
  assert_eq!(first.id, 1);

  let second: GeneratedId = index_symbol("toolu-orm", "src/main.rs", "t1")
    .returning(&SYMBOL_ID)
    .fetch_one(&conn)?;
  assert_eq!(second.id, 2, "a distinct row takes the next id");

  let again: GeneratedId = index_symbol("toolu-orm", "src/lib.rs", "t2")
    .returning(&SYMBOL_ID)
    .fetch_one(&conn)?;
  assert_eq!(
    again.id, first.id,
    "DO UPDATE writes the stored row, so its id survives"
  );

  let rows: Vec<CodeSymbol> = SelectBuilder::new("code_symbols")
    .columns_raw(&SYMBOL_COLUMNS)
    .order_by(SYMBOL_ID.asc())
    .fetch_all(&conn)?;
  assert_eq!(rows.len(), 2, "the conflicting insert updated, not added");
  assert_eq!(
    rows.first().map(|row| row.indexed_at.as_str()),
    Some("t2"),
    "and it took the proposed timestamp"
  );
  Ok(())
}

#[test]
fn or_replace_allocates_a_new_id_where_do_update_kept_it() -> TestResult {
  let conn = db::setup_db()?;
  index_symbol("toolu-orm", "src/lib.rs", "t1").execute(&conn)?;

  InsertBuilder::new("code_symbols")
    .set(&REPO, "toolu-orm")
    .set(&PATH, "src/lib.rs")
    .set(&INDEXED_AT, "t2")
    .or_replace()
    .execute(&conn)?;

  let row: CodeSymbol = SelectBuilder::new("code_symbols")
    .columns_raw(&SYMBOL_COLUMNS)
    .fetch_one(&conn)?;
  assert_eq!(
    row.id, 2,
    "the delete/re-insert gives the row a brand new id"
  );
  Ok(())
}

#[test]
fn fetch_optional_is_none_when_do_nothing_suppressed_the_write() -> TestResult {
  let conn = db::setup_db()?;
  seed_memory(&conn)?;

  let suppressed: Option<GeneratedId> = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .on_conflict(OnConflict::column(&MEMORY_ID).do_nothing())
    .returning(&MEMORY_ID)
    .fetch_optional(&conn)?;
  assert!(suppressed.is_none(), "no row was written, so none returned");
  Ok(())
}

#[test]
fn fetch_one_without_a_returning_clause_reports_not_found() -> TestResult {
  let conn = db::setup_db()?;

  let outcome = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m9")
    .set(&BODY, "fresh")
    .fetch_one::<GeneratedId>(&conn);

  let Err(QueryError::NotFound { table }) = outcome else {
    return Err("a statement that projects nothing must report NotFound".into());
  };
  assert_eq!(table, "memories");

  let stored = SelectBuilder::new("memories")
    .filter(MEMORY_ID.eq("m9"))
    .count(&conn)?;
  assert_eq!(stored, 1, "the row was still inserted");
  Ok(())
}

#[test]
fn execute_refuses_a_returning_statement_on_rusqlite() -> TestResult {
  let conn = db::setup_db()?;

  let outcome = index_symbol("toolu-orm", "src/lib.rs", "t1")
    .returning(&SYMBOL_ID)
    .execute(&conn);

  let Err(QueryError::Driver(error)) = outcome else {
    return Err("rusqlite refuses to `execute` a row-producing statement".into());
  };
  assert!(
    error.to_string().contains("Execute returned results"),
    "unexpected driver error: {error}"
  );
  Ok(())
}

#[test]
fn fetch_all_returns_the_one_projected_row_and_an_empty_vector_when_suppressed() -> TestResult {
  let conn = db::setup_db()?;

  let written: Vec<GeneratedId> = index_symbol("toolu-orm", "src/lib.rs", "t1")
    .returning(&SYMBOL_ID)
    .fetch_all(&conn)?;
  assert_eq!(written, vec![GeneratedId { id: 1 }]);

  let suppressed: Vec<GeneratedId> = InsertBuilder::new("code_symbols")
    .set(&REPO, "toolu-orm")
    .set(&PATH, "src/lib.rs")
    .set(&INDEXED_AT, "t2")
    .on_conflict(OnConflict::column(&REPO).and_column(&PATH).do_nothing())
    .returning(&SYMBOL_ID)
    .fetch_all(&conn)?;
  assert!(suppressed.is_empty(), "DO NOTHING projects no row");
  Ok(())
}
