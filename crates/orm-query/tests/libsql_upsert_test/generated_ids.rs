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

#[tokio::test]
async fn returning_hands_back_the_generated_id_and_keeps_it_across_a_conflict() -> TestResult {
  let conn = db::setup_db().await?;

  let first: GeneratedId = index_symbol("toolu-orm", "src/lib.rs", "t1")
    .returning(&SYMBOL_ID)
    .fetch_one(&conn)
    .await?;
  assert_eq!(first.id, 1);

  let second: GeneratedId = index_symbol("toolu-orm", "src/main.rs", "t1")
    .returning(&SYMBOL_ID)
    .fetch_one(&conn)
    .await?;
  assert_eq!(second.id, 2, "a distinct row takes the next id");

  let again: GeneratedId = index_symbol("toolu-orm", "src/lib.rs", "t2")
    .returning(&SYMBOL_ID)
    .fetch_one(&conn)
    .await?;
  assert_eq!(
    again.id, first.id,
    "DO UPDATE writes the stored row, so its id survives"
  );

  let rows: Vec<CodeSymbol> = SelectBuilder::new("code_symbols")
    .columns_raw(&SYMBOL_COLUMNS)
    .order_by(SYMBOL_ID.asc())
    .fetch_all(&conn)
    .await?;
  assert_eq!(rows.len(), 2, "the conflicting insert updated, not added");
  assert_eq!(
    rows.first().map(|row| row.indexed_at.as_str()),
    Some("t2"),
    "and it took the proposed timestamp"
  );
  Ok(())
}

#[tokio::test]
async fn or_replace_allocates_a_new_id_where_do_update_kept_it() -> TestResult {
  let conn = db::setup_db().await?;
  index_symbol("toolu-orm", "src/lib.rs", "t1")
    .execute(&conn)
    .await?;

  InsertBuilder::new("code_symbols")
    .set(&REPO, "toolu-orm")
    .set(&PATH, "src/lib.rs")
    .set(&INDEXED_AT, "t2")
    .or_replace()
    .execute(&conn)
    .await?;

  let row: CodeSymbol = SelectBuilder::new("code_symbols")
    .columns_raw(&SYMBOL_COLUMNS)
    .fetch_one(&conn)
    .await?;
  assert_eq!(
    row.id, 2,
    "the delete/re-insert gives the row a brand new id"
  );
  Ok(())
}

#[tokio::test]
async fn fetch_optional_is_none_when_do_nothing_suppressed_the_write() -> TestResult {
  let conn = db::setup_db().await?;
  seed_memory(&conn).await?;

  let suppressed: Option<GeneratedId> = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m1")
    .set(&BODY, "incoming")
    .on_conflict(OnConflict::column(&MEMORY_ID).do_nothing())
    .returning(&MEMORY_ID)
    .fetch_optional(&conn)
    .await?;
  assert!(suppressed.is_none(), "no row was written, so none returned");
  Ok(())
}

#[tokio::test]
async fn fetch_one_without_a_returning_clause_reports_not_found() -> TestResult {
  let conn = db::setup_db().await?;

  let outcome = InsertBuilder::new("memories")
    .set(&MEMORY_ID, "m9")
    .set(&BODY, "fresh")
    .fetch_one::<GeneratedId>(&conn)
    .await;

  let Err(QueryError::NotFound { table }) = outcome else {
    return Err("a statement that projects nothing must report NotFound".into());
  };
  assert_eq!(table, "memories");

  let stored = SelectBuilder::new("memories")
    .filter(MEMORY_ID.eq("m9"))
    .count(&conn)
    .await?;
  assert_eq!(stored, 1, "the row was still inserted");
  Ok(())
}
